extern crate libc;
extern crate nix;
extern crate rand;
extern crate rqc_core;

use nix::errno::Errno;
use nix::fcntl::OFlag;
use nix::sys::mman::{mmap, shm_open, shm_unlink, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execv, fork, ftruncate, ForkResult};
use rand::rngs::SmallRng;
use rand::{FromEntropy, Rng};
use rqc_core::{Backoff, ClientStatus, Comm, ServerStatus, TestStatus};
use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

pub struct Rqc {
    shm_total_bytes: usize,
    shm_path: String,
    target_byte_pool_size: usize,
}

impl Rqc {
    pub fn new() -> Self {
        Self {
            shm_total_bytes: 1024,
            shm_path: String::from("/RQC"),
            target_byte_pool_size: 256,
        }
    }

    pub fn build(&self) -> () {
        let cargo_path = env!("CARGO");

        let mut rustflags: String = "-C debug-assertions \
                                     -C overflow_checks \
                                     -C opt-level=3 \
                                     -C target-cpu=native"
            .to_string();

        // add user provided flags
        let other_flags = ::std::env::var("RUSTFLAGS").unwrap_or_default();
        if !other_flags.is_empty() {
            rustflags.push_str(" ");
            rustflags.push_str(&other_flags);
        }

        let mut cmd = ::std::process::Command::new(cargo_path);
        cmd.arg("build").arg("--release");

        let status = cmd.env("RUSTFLAGS", &rustflags).status().unwrap();
        ::std::process::exit(status.code().unwrap_or(1));
    }

    pub fn run(&self, target: &Path) -> () {
        let def_file_mode = Mode::S_IRUSR
            | Mode::S_IWUSR
            | Mode::S_IRGRP
            | Mode::S_IWGRP
            | Mode::S_IROTH
            | Mode::S_IWOTH;
        let _ = shm_unlink(self.shm_path.as_str());
        let memfd = shm_open(
            self.shm_path.as_str(),
            OFlag::O_CREAT | OFlag::O_RDWR,
            def_file_mode,
        )
        .expect("failed to open shared memory");
        if let Err(e) = ftruncate(memfd, self.shm_total_bytes as i64) {
            shm_unlink(self.shm_path.as_str()).expect("failed to unlink opened shm");
            println!(
                "could not truncate shared memory to appropriate size: {}",
                e
            );
            ::std::process::exit(1);
        }
        let ptr: *mut libc::c_void = unsafe {
            mmap(
                0 as *mut libc::c_void,
                1024,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                memfd,
                0,
            )
            .expect("could not memory map shared memory file")
        };
        let mut comm = Comm::new(ptr, self.shm_total_bytes);
        // NOTE(blt) -- okay, now, at this point we actually need to fork/exec
        // and get a child process to do a similar mmap dance. Also, we should
        // hide this inside a proper type. Initial goal: just get a handshake
        // going between the client (test target) and server (test runner).

        let c_path = CString::new(
            target
                .as_os_str()
                .to_str()
                .expect("path contains invalid unicode characters"),
        )
        .expect("unable to coerce path into c-style string");

        let mut restarts = 0;
        let mut passed = 0;
        let mut skipped = 0;
        let mut failed = 0;
        let mut crash_failure = 0;
        let mut insufficient_bytes = 0;
        let mut test_cases = 0;

        let ui_delay = Duration::from_secs(1);
        let mut start = Instant::now();

        let mut restart_target = false;
        let mut exit_status = None; // If exit status is ever Some then we quit
        loop {
            if exit_status.is_some() {
                break;
            }
            match fork() {
                Ok(ForkResult::Parent { child, .. }) => {
                    let mut rng = SmallRng::from_entropy();
                    let mut bytes: Vec<u8> = Vec::with_capacity(self.target_byte_pool_size);
                    for _ in 0..self.target_byte_pool_size {
                        bytes.push(0);
                    }

                    let mut backoff = Backoff::default();
                    loop {
                        backoff.delay();
                        if start.elapsed() >= ui_delay {
                            start = Instant::now();
                            println!(
                                "TestCases: {} Restarts: {} Passed: {} Skipped: {} Failed: {} InsufficientBytes: {} CrashFail: {}",
                                test_cases, restarts, passed, skipped, failed, insufficient_bytes, crash_failure
                            );
                        }
                        // TODO(blt) -- if the child dies we need to be sure not
                        // to leave an orphan around
                        match waitpid(child, Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED)) {
                            Ok(status) => match status {
                                WaitStatus::StillAlive => {
                                    match comm.client_status() {
                                        ClientStatus::Default => {
                                            // nothing to do
                                        }
                                        ClientStatus::Ready => match comm.server_status() {
                                            ServerStatus::Default => {
                                                backoff.reset();
                                                for b in bytes.iter_mut() {
                                                    *b = rng.gen::<u8>();
                                                }
                                                let _ = comm.write(&mut bytes).expect(
                                                    "unable to write random bytes to target",
                                                );
                                                test_cases += 1;
                                                comm.server_ready();
                                            }
                                            ServerStatus::Ready => {}
                                        },
                                        ClientStatus::Test(test_status) => {
                                            match test_status {
                                                TestStatus::Passed => passed += 1,
                                                TestStatus::Skipped => skipped += 1,
                                                TestStatus::Failed => failed += 1,
                                                TestStatus::InsufficientBytes => {
                                                    insufficient_bytes += 1
                                                }
                                            }
                                            comm.client_reset();
                                            comm.server_reset();
                                        }
                                    }
                                }
                                WaitStatus::Exited(_, status) => {
                                    if status != 0 {
                                        println!("target exited with non-zero status: {}", status);
                                        crash_failure += 1;
                                        restart_target = true;
                                    } else {
                                        if test_cases == 0 {
                                            println!(
                                                "target exited before a test could be given to it"
                                            );
                                            exit_status = Some(1); // TODO(blt) -- have well-defined exit status meanings
                                        } else {
                                            restart_target = true;
                                        }
                                    }
                                }
                                s => {
                                    println!("target finished with status: {:?}", s);
                                }
                            },
                            Err(e) => match e.as_errno() {
                                Some(Errno::ECHILD) => {}
                                _ => {
                                    println!("waiting on target failed with: {}", e);
                                    exit_status = Some(1) // TODO(blt) -- have well-defined exit status meanings
                                }
                            },
                        }
                        if restart_target || exit_status.is_some() {
                            restarts += 1;
                            break;
                        }
                    }
                }
                Ok(ForkResult::Child) => {
                    // TODO(blt) for some reason the args aren't getting passed to the child
                    execv(
                        &c_path,
                        &[
                            c_path.clone(),
                            CString::new(self.shm_path.as_str()).unwrap(),
                        ],
                    )
                    .expect("could not execv");
                }
                Err(_) => {
                    println!("Unable to fork target");
                    ::std::process::exit(1);
                }
            }
        } // end loop
        shm_unlink(self.shm_path.as_str()).expect("failed to unlink opened shm");
        ::std::process::exit(exit_status.unwrap());
    }
}
