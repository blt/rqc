extern crate rqc_core;

use rqc_core::{Arbitrary, BufferOpError, FiniteByteBuffer, Rqc, RqcBuild, TestResult};

fn check(buf: &mut FiniteByteBuffer) -> Result<TestResult, BufferOpError> {
    let s: String = Arbitrary::arbitrary(buf)?;
    let repeats: u8 = Arbitrary::arbitrary(buf)?;
    let repeats: usize = repeats as usize;

    if s == "hi!" {
        return Ok(TestResult::Failed);
    }
    if let Some(rpt_len) = s.len().checked_mul(repeats) {
        let res = s.repeat(repeats);
        if res.len() != rpt_len {
            return Ok(TestResult::Failed);
        }
    }
    return Ok(TestResult::Passed);
}

fn main() {
    let rqc: Rqc = RqcBuild::new().ui_update_seconds(60).build();
    rqc.run(check)
}
