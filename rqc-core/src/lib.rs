mod arbitrary;
mod byte_buffer;

pub use crate::arbitrary::*;
pub use crate::byte_buffer::*;
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
    pub fn run<F>(self, closure: F)
    where
        F: Fn(&mut FiniteByteBuffer) -> Result<TestResult, BufferOpError>,
    {
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
