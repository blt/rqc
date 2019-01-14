extern crate bh_alloc;
extern crate rqc_core;

#[global_allocator]
static ALLOC: bh_alloc::fuzz::BumpAlloc = bh_alloc::fuzz::BumpAlloc::INIT;

use rqc_core::{Arbitrary, BufferOpError, FiniteByteBuffer, Rqc, RqcBuild, TestResult};
use std::env;

fn check(buf: &mut FiniteByteBuffer) -> Result<TestResult, BufferOpError> {
    let s: String = Arbitrary::arbitrary(buf)?;
    let repeats: u8 = Arbitrary::arbitrary(buf)?;
    let repeats: usize = repeats as usize;

    if let Some(rpt_len) = s.len().checked_mul(repeats) {
        let res = s.repeat(repeats);
        if res.len() != rpt_len {
            return Ok(TestResult::Failed);
        }
    }
    return Ok(TestResult::Passed);
}

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();
    let shm_path = args
        .next()
        .expect("must have a path to shm for communication with server");

    let rqc: Rqc = RqcBuild::new().build();
    rqc.run(&shm_path, check)
}
