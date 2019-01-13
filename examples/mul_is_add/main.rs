extern crate rqc_core;

use rqc_core::{Arbitrary, BufferOpError, FiniteByteBuffer, Rqc, RqcBuild, TestResult};
use std::env;

fn check(buf: &mut FiniteByteBuffer) -> Result<TestResult, BufferOpError> {
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
    let mut args = env::args();
    let _ = args.next().unwrap();
    let shm_path = args
        .next()
        .expect("must have a path to shm for communication with server");

    println!("SHM_PATH: {}", shm_path);
    let rqc: Rqc = RqcBuild::new().build();
    rqc.run(&shm_path, check)
}
