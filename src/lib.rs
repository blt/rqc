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
use rqc_core::{Comm, Stat};
use std::ffi::CString;
use std::io::Write;
use std::path::Path;

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

        match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                let mut rng = SmallRng::from_entropy();
                let mut bytes: Vec<u8> = Vec::with_capacity(self.target_byte_pool_size);
                for _ in 0..self.target_byte_pool_size {
                    bytes.push(0);
                }
                let mut st = 0;

                // Okay, here's the server/target protocol. The client signals
                // that it's ready and the server then writes random bytes down
                // the comm, possibly doing a shrink if our pass/fail/skip
                // levels have changed to indicate a failure. The server will
                // switch the client to a non-ready state, meaning that the
                // client only ever switches itself to ready when the server has
                // previously noted that it's time has come.
                //
                // It's late and I"m tired. A diagram will explain this
                // better. What we want to avoid is stalling on signals or some
                // such.

                loop {
                    // NOTE(blt) -- this loop is not going to work. The client
                    // needs to be able to advertise several states:
                    //
                    // * the client is ready to receive a byte blog
                    // * the client ran the byte blob and has results
                    // * the client is still executing the byte blob
                    // * the client failed
                    //
                    // This means extending Comm, I think, and changing this
                    // loop. Tomorrow.
                    //
                    // Also of note, there's no shrinking yet. No shrinking, no
                    // way to replay and all the indexes are just wacky.

                    // println!(
                    //     "PASSED: {}\tSKIPPED: {}\tFAILED: {}",
                    //     comm.stat(Stat::PassedTests),
                    //     comm.stat(Stat::SkippedTests),
                    //     comm.stat(Stat::FailedTests)
                    // );
                    match waitpid(child, Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED)) {
                        Ok(status) => match status {
                            WaitStatus::StillAlive => {
                                if comm.is_client_ready() {
                                    for b in bytes.iter_mut() {
                                        *b = rng.gen::<u8>();
                                    }
                                    let l = comm
                                        .write(&mut bytes)
                                        .expect("unable to write random bytes to target");
                                    println!("WROTE {} BYTES TO TARGET", l);
                                    comm.client_unready();
                                    comm.server_ready();
                                }
                                continue;
                            }
                            WaitStatus::Exited(_, exit_status) => {
                                if exit_status != 0 {
                                    println!("target exited with non-zero status: {}", exit_status);
                                    st = exit_status;
                                } else {
                                    continue;
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
                                st = 1;
                            }
                        },
                    }
                    break;
                }
                shm_unlink(self.shm_path.as_str()).expect("failed to unlink opened shm");
                ::std::process::exit(st);
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
    }
}
