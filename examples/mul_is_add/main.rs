extern crate rqc_core;

use rqc_core::{Arbitrary, BufferOpError, ByteBuffer, Rqc, RqcBuild, TestResult};

fn check(buf: &mut ByteBuffer) -> Result<TestResult, BufferOpError> {
    let lhs: u8 = Arbitrary::arbitrary(buf)?;
    let rhs: u8 = Arbitrary::arbitrary(buf)?;

    let mul = lhs * rhs;
    let mut add_mul = 0;
    for _ in 0..rhs {
        add_mul += lhs;
    }

    if add_mul != mul {
        return Ok(TestResult::Failed);
    }
    Ok(TestResult::Passed)
}

fn main() {
    let rqc: Rqc = RqcBuild::new()
        .runtime_seconds(60)
        .maximum_tests(100_000_000)
        .build();
    rqc.run(check)
}
