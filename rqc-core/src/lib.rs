extern crate arbitrary;
extern crate rand;

use arbitrary::Unstructured;
pub use arbitrary::{Arbitrary, FBError, FiniteBuffer};
use rand::rngs::SmallRng;
use rand::{FromEntropy, Rng};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time;

#[derive(Default)]
pub struct RqcBuild {
    maximum_tests: Option<u64>,
    runtime_seconds: Option<u64>,
}

impl RqcBuild {
    pub fn new() -> RqcBuild {
        RqcBuild::default()
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
            runtime_seconds: self.runtime_seconds.unwrap_or(60),
            maximum_tests: self.maximum_tests.unwrap_or(100_000),
        }
    }
}

pub const TOTAL_BYTES: usize = 32_768;

pub struct Rqc {
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
        F: Fn(&mut FiniteBuffer<'_>) -> Result<TestResult, FBError>,
    {
        let ui = ::std::thread::spawn(ui);

        let mut bytes: Vec<u8> = Vec::with_capacity(TOTAL_BYTES);
        let mut rng = SmallRng::from_entropy();
        for _ in 0..TOTAL_BYTES {
            bytes.push(rng.gen::<u8>())
        }
        let mut buf = FiniteBuffer::new(&bytes)
            .expect("could not create arbitrary buffer")
            .container_size_limit(128);

        let start = time::Instant::now();
        let mut current_test_iteration = 0;
        while current_test_iteration < self.maximum_tests {
            let cur = time::Instant::now();
            if cur.duration_since(start).as_secs() > self.runtime_seconds {
                break;
            }

            match closure(&mut buf) {
                Ok(TestResult::Passed) => {
                    PASSED_TESTS.fetch_add(1, Ordering::Relaxed);
                }
                Ok(TestResult::Failed) => {
                    // NOTE(blt) -- will need to record the offset at the start
                    // and end of the test then shrink _that_, else we're just
                    // kicking out a new test entirely
                    FAILED_TESTS.fetch_add(1, Ordering::Relaxed);
                }
                Ok(TestResult::Skipped) => {
                    SKIPPED_TESTS.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                Err(FBError::InsufficientBytes) => {
                    buf.reset();
                    buf.shift_right(1).expect("could not shift buffer");
                }
            }
            current_test_iteration += 1;
        }
        SHUTDOWN.store(true, Ordering::Relaxed);
        ui.join().unwrap();
    }
}
