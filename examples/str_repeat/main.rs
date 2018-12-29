extern crate rqc_core;

use rqc_core::{Arbitrary, FBError, FiniteBuffer, Rqc, RqcBuild, TestResult};
use std::str;

fn check(buf: &mut FiniteBuffer<'_>) -> Result<TestResult, FBError> {
    let vs: Vec<u8> = Arbitrary::arbitrary(buf)?;

    let s: &str = if let Ok(s) = str::from_utf8(&vs) {
        s
    } else {
        return Ok(TestResult::Skipped);
    };
    let repeats: u16 = Arbitrary::arbitrary(buf)?;
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
    let rqc: Rqc = RqcBuild::new()
        .runtime_seconds(60)
        .maximum_tests(10_000)
        .build();
    rqc.run(check)
}
