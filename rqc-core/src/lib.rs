extern crate rand;

mod arbitrary;
mod byte_buffer;

pub use crate::arbitrary::*;
pub use crate::byte_buffer::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time;

#[derive(Default)]
pub struct RqcBuild {
    rng_seed: Option<u64>,
    byte_pool_capacity: Option<u32>,
    maximum_tests: Option<u64>,
    runtime_seconds: Option<u64>,
}

impl RqcBuild {
    pub fn new() -> RqcBuild {
        RqcBuild::default()
    }

    pub fn rng_seed(mut self, rng_seed: u64) -> RqcBuild {
        self.rng_seed = Some(rng_seed);
        self
    }

    pub fn byte_pool_capacity(mut self, byte_pool_capacity: u32) -> RqcBuild {
        self.byte_pool_capacity = Some(byte_pool_capacity);
        self
    }

    pub fn maximum_tests(mut self, maximum_tests: u64) -> RqcBuild {
        self.maximum_tests = Some(maximum_tests);
        self
    }

    pub fn runtime_seconds(mut self, seconds: u64) -> RqcBuild {
        self.runtime_seconds = Some(seconds);
        self
    }

    pub fn build(self) -> Rqc {
        Rqc {
            byte_pool_capacity: self.byte_pool_capacity.unwrap_or(32_768) as usize,
            rng_seed: self.rng_seed.unwrap_or_else(|| {
                time::SystemTime::now()
                    .duration_since(time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }),
            runtime_seconds: self.runtime_seconds.unwrap_or(60),
            maximum_tests: self.maximum_tests.unwrap_or(100_000),
        }
    }
}

pub const TOTAL_BYTES: usize = 32_768;

pub struct Rqc {
    rng_seed: u64,
    byte_pool_capacity: usize,
    maximum_tests: u64,
    runtime_seconds: u64,
}

pub enum TestResult {
    Passed,
    Skipped,
    Failed,
}

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static SKIPPED_TESTS: AtomicUsize = AtomicUsize::new(0);
static FAILED_TESTS: AtomicUsize = AtomicUsize::new(0);
static PASSED_TESTS: AtomicUsize = AtomicUsize::new(0);

fn ui() -> () {
    let slp = std::time::Duration::from_millis(1_000);
    let mut global_skipped_tests = 0;
    let mut global_failed_tests = 0;
    let mut global_passed_tests = 0;
    while !SHUTDOWN.load(Ordering::Relaxed) {
        ::std::thread::sleep(slp);
        let skipped_tests = SKIPPED_TESTS.swap(0, Ordering::Relaxed);
        let failed_tests = FAILED_TESTS.swap(0, Ordering::Relaxed);
        let passed_tests = PASSED_TESTS.swap(0, Ordering::Relaxed);

        global_skipped_tests += skipped_tests;
        global_failed_tests += failed_tests;
        global_passed_tests += passed_tests;

        println!(
            "\rpassed: {}\tskipped: {}\tfailed: {}",
            global_passed_tests, global_skipped_tests, global_failed_tests,
        );
        println!(
            "\rpassed/sec: {}\tskipped/sec: {}\tfailed/sec: {}",
            passed_tests, skipped_tests, failed_tests,
        );
        println!("");
    }
}

impl Rqc {
    pub fn run<F>(self, closure: F)
    where
        F: Fn(&mut ByteBuffer) -> Result<TestResult, BufferOpError>,
    {
        let ui = ::std::thread::spawn(ui);

        let mut buf = ByteBuffer::new(self.byte_pool_capacity, self.rng_seed)
            .expect("could not create arbitrary buffer");
        let mut realloc_buffer = false;
        let start = time::Instant::now();
        let mut current_test_iteration = 0;
        while current_test_iteration < self.maximum_tests {
            if realloc_buffer {
                // TODO(blt) -- force byte buffer to initialize its own byte slice
                buf.hard_reset();
                realloc_buffer = false;
            }

            let cur = time::Instant::now();
            if cur.duration_since(start).as_secs() > self.runtime_seconds {
                break;
            }

            let offset = buf.offset;
            loop {
                match closure(&mut buf) {
                    Ok(TestResult::Passed) => {
                        PASSED_TESTS.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    Ok(TestResult::Skipped) => {
                        // TODO(blt) keep some type of skip counter per test
                        // iteration to fidget with the offset in case we get
                        // stuck in a skip-loop
                        SKIPPED_TESTS.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    Ok(TestResult::Failed) => {
                        // NOTE(blt) -- we need to have some way of recording the
                        // failure, whether it shrinky-dinks or not
                        FAILED_TESTS.fetch_add(1, Ordering::Relaxed);
                        if buf.shrink_from(offset) == 0 {
                            buf.soft_reset();
                            match buf.shift_right() {
                                Ok(_) => {}
                                Err(BufferOpError::ShiftWrapAround) => {
                                    realloc_buffer = true;
                                    break;
                                }
                                Err(_) => unreachable!(),
                            }
                        }
                    }
                    Err(BufferOpError::InsufficientBytes) => {
                        // We could also choose to increase the virtual length
                        // here, instead of shifting out. For example, consider
                        // if the user were trying to create 8k u8s but the
                        // virtual_len were 10. This would never succeed.
                        buf.soft_reset();
                        match buf.shift_right() {
                            Ok(_) => {}
                            Err(BufferOpError::ShiftWrapAround) => {
                                realloc_buffer = true;
                                break;
                            }
                            Err(_) => unreachable!(),
                        }
                    }
                    Err(_) => unreachable!(),
                }
            }
            current_test_iteration += 1;
        }
        SHUTDOWN.store(true, Ordering::Relaxed);
        ui.join().unwrap();
    }
}
