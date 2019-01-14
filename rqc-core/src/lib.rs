extern crate libc;
extern crate nix;

mod arbitrary;
mod backoff;
mod byte_buffer;
mod comm;

pub use crate::arbitrary::*;
pub use crate::backoff::*;
pub use crate::byte_buffer::*;
pub use crate::comm::*;
use nix::fcntl::OFlag;
use nix::sys::mman::{mmap, shm_open, MapFlags, ProtFlags};
use nix::sys::stat::{fstat, Mode};
use std::io::Read;

#[derive(Default)]
pub struct RqcBuild {
    byte_pool_capacity: Option<u32>,
}

impl RqcBuild {
    pub fn new() -> RqcBuild {
        RqcBuild::default()
    }

    pub fn byte_pool_capacity(mut self, byte_pool_capacity: u32) -> RqcBuild {
        self.byte_pool_capacity = Some(byte_pool_capacity);
        self
    }

    pub fn build(self) -> Rqc {
        Rqc {
            byte_pool_capacity: self.byte_pool_capacity.unwrap_or(1_048_576) as usize,
        }
    }
}

pub const TOTAL_BYTES: usize = 32_768;

pub struct Rqc {
    byte_pool_capacity: usize,
}

pub enum TestResult {
    Passed,
    Skipped,
    Failed,
}

impl Rqc {
    pub fn run<F>(self, shm_path: &str, closure: F)
    where
        F: Fn(&mut FiniteByteBuffer) -> Result<TestResult, BufferOpError>,
    {
        let def_file_mode = Mode::S_IRUSR
            | Mode::S_IWUSR
            | Mode::S_IRGRP
            | Mode::S_IWGRP
            | Mode::S_IROTH
            | Mode::S_IWOTH;
        let memfd = shm_open(shm_path, OFlag::O_CREAT | OFlag::O_RDWR, def_file_mode)
            .expect("failed to open shared memory");
        let total_bytes = fstat(memfd).expect("could not fstat shm file").st_size as usize;

        let ptr: *mut libc::c_void = unsafe {
            mmap(
                0 as *mut libc::c_void,
                total_bytes,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                memfd,
                0,
            )
            .expect("could not memory map shared memory file")
        };
        let mut comm = Comm::new(ptr, total_bytes);

        // NOTE(blt)
        //
        // instrumentation that is wanted
        //  - offsets in the buffer at the start of test runs
        //  - offset at the end of a test run
        //  - broad result -- pass/fail/skip -- from tests
        //  - switch states in the interpreter loop, this being either
        //    macro derived or user supplied

        let mut byte_buf = Vec::with_capacity(self.byte_pool_capacity);
        for _ in 0..self.byte_pool_capacity {
            byte_buf.push(0);
        }

        let mut backoff = Backoff::default();
        loop {
            comm.client_ready();
            loop {
                match comm.server_status() {
                    ServerStatus::Ready => {
                        backoff.reset();
                        break;
                    }
                    _ => backoff.delay(),
                }
            }

            match comm.read(&mut byte_buf) {
                Err(_) => ::std::process::exit(0),
                Ok(0) => continue,
                Ok(_) => {}
            }
            let mut buf = FiniteByteBuffer::new(&byte_buf);
            match closure(&mut buf) {
                Ok(TestResult::Passed) => {
                    comm.client_test_status(TestStatus::Passed);
                }
                Ok(TestResult::Skipped) => {
                    comm.client_test_status(TestStatus::Skipped);
                }
                Ok(TestResult::Failed) => {
                    comm.client_test_status(TestStatus::Failed);
                }
                Err(BufferOpError::InsufficientBytes) => {
                    comm.client_test_status(TestStatus::InsufficientBytes);
                }
            }
            loop {
                match comm.server_status() {
                    ServerStatus::Default => {
                        backoff.reset();
                        break;
                    }
                    _ => backoff.delay(),
                }
            }
        }
    }
}
