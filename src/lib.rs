extern crate libc;
extern crate nix;

use nix::errno::Errno;
use nix::fcntl::OFlag;
use nix::sys::mman::{mmap, shm_open, shm_unlink, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execv, fork, ftruncate, ForkResult};
use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::path::Path;

pub struct Rqc {}

impl Rqc {
    pub fn new() -> Self {
        Self {}
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
        let shm_path = "/RQC";
        let memfd = shm_open(shm_path, OFlag::O_CREAT | OFlag::O_RDWR, def_file_mode)
            .expect("failed to open shared memory");
        if let Err(e) = ftruncate(memfd, 1024) {
            shm_unlink(shm_path).expect("failed to unlink opened shm");
            println!("could not truncate shared memory to appropriate size");
            ::std::process::exit(1);
        }
        let ptr = unsafe {
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
                let mut st = 0;
                loop {
                    match waitpid(child, Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED)) {
                        Ok(status) => match status {
                            WaitStatus::StillAlive => {
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
                shm_unlink(shm_path).expect("failed to unlink opened shm");
                ::std::process::exit(st);
            }
            Ok(ForkResult::Child) => {
                // TODO(blt) for some reason the args aren't getting passed to the child
                execv(&c_path, &[c_path.clone(), CString::new(shm_path).unwrap()])
                    .expect("could not execv");
            }
            Err(_) => {
                println!("Unable to fork target");
                ::std::process::exit(1);
            }
        }
        // execv(&c_path, &[c_path.clone(), CString::new(shm_path).unwrap()])
        //     .expect("could not execv");
    }
}
