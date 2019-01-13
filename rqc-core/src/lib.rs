extern crate libc;
extern crate nix;

mod arbitrary;
mod byte_buffer;
mod comm;

pub use crate::arbitrary::*;
pub use crate::byte_buffer::*;
pub use crate::comm::*;
use nix::fcntl::OFlag;
use nix::sys::mman::{mmap, shm_open, MapFlags, ProtFlags};
use nix::sys::stat::{fstat, Mode};
use std::io::Read;
use std::{io, time};

#[derive(Default)]
pub struct RqcBuild {
    byte_pool_capacity: Option<u32>,
    ui_update_seconds: Option<u64>,
}

impl RqcBuild {
    pub fn new() -> RqcBuild {
        RqcBuild::default()
    }

    pub fn byte_pool_capacity(mut self, byte_pool_capacity: u32) -> RqcBuild {
        self.byte_pool_capacity = Some(byte_pool_capacity);
        self
    }

    pub fn ui_update_seconds(mut self, seconds: u64) -> RqcBuild {
        self.ui_update_seconds = Some(seconds);
        self
    }

    pub fn build(self) -> Rqc {
        Rqc {
            byte_pool_capacity: self.byte_pool_capacity.unwrap_or(1_048_576) as usize,
            ui_update_seconds: self.ui_update_seconds.unwrap_or(60),
        }
    }
}

pub const TOTAL_BYTES: usize = 32_768;

pub struct Rqc {
    byte_pool_capacity: usize,
    ui_update_seconds: u64,
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

        let start = time::Instant::now();
        let mut passed_tests = 0;
        let mut skipped_tests = 0;
        let mut failed_tests = 0;

        let mut io_buf = Vec::with_capacity(self.byte_pool_capacity);
        for _ in 0..self.byte_pool_capacity {
            io_buf.push(0);
        }
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        comm.client_ready();
        while !comm.is_server_ready() {}
        println!("SERVER IS READY");

        loop {
            match handle.read(&mut io_buf) {
                Err(_) => ::std::process::exit(0),
                Ok(0) => continue,
                Ok(_) => {}
            }
            let mut buf = FiniteByteBuffer::new(&io_buf);
            loop {
                let cur = time::Instant::now();
                if cur.duration_since(start).as_secs() > self.ui_update_seconds {
                    println!(
                        "\rpassed: {}\tskipped: {}\tfailed: {}",
                        passed_tests, skipped_tests, failed_tests,
                    );
                }
                match closure(&mut buf) {
                    Ok(TestResult::Passed) => {
                        passed_tests += 1;
                        continue;
                    }
                    Ok(TestResult::Skipped) => {
                        skipped_tests += 1;
                        continue;
                    }
                    Ok(TestResult::Failed) => {
                        failed_tests += 1;
                        break;
                    }
                    Err(BufferOpError::InsufficientBytes) => {
                        println!(
                            "\rpassed: {}\tskipped: {}\tfailed: {}",
                            passed_tests, skipped_tests, failed_tests,
                        );
                        ::std::process::exit(0);
                    }
                }
            }
        }
    }
}
