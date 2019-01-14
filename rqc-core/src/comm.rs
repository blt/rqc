use std::{io, mem, ptr};

pub struct Comm {
    ptr: *mut u64,
    //    len: usize,
}

const SERVER_STATUS_OFFSET: isize = 0;
const CLIENT_STATUS_OFFSET: isize = 1;
const BYTE_POOL_SIZE_OFFSET: isize = 2;
const BYTE_POOL_OFFSET: isize = 3;

const SERVER_DEFAULT: u64 = 0;
const SERVER_READY: u64 = 1;

const CLIENT_DEFAULT: u64 = 0;
const CLIENT_READY: u64 = 1;
const CLIENT_TEST_PASSED: u64 = 2;
const CLIENT_TEST_SKIPPED: u64 = 3;
const CLIENT_TEST_FAILED: u64 = 4;
const CLIENT_TEST_INSUFFICIENT_BYTES: u64 = 5;

#[derive(Debug)]
pub enum ClientStatus {
    Default,
    Ready,
    Test(TestStatus),
}

#[derive(Debug)]
pub enum ServerStatus {
    Default,
    Ready,
}

#[derive(Debug)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    InsufficientBytes,
}

/*

server

  wait(CLIENT_READY) | detect exit
    -> set comm
    -> set(SERVER_READY)
    -> wait(CLIENT_TEST_COMPLETE) | signal failure
    -> set(CLIENT_DEFAULT)
    -> set(SERVER_DEFAULT)
    -> if failure, shrink else random

target

  0: set(CLIENT_READY)
      -> wait(SERVER_READY)
      -> test run, signals
      -> set(CLIENT_TEST_PASSED) | set(CLIENT_TEST_SKIPPED) |
         set(CLIENT_TEST_FAILED) | set(CLIENT_TEST_INSUFFICIENT_BYTES) |
         exit 0 (skipped) | exit non-zero (failed)
      -> goto 0

*/

impl io::Write for Comm {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let total_bytes = buf.len();
        // TODO(blt) -- check that the buffer being written isn't too big for the space we have
        unsafe {
            *self.ptr.offset(BYTE_POOL_SIZE_OFFSET) = total_bytes as u64;
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.ptr.offset(BYTE_POOL_OFFSET) as *mut u8,
                total_bytes,
            );
        }
        Ok(total_bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Read for Comm {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let total_bytes =
                *self.ptr.offset(BYTE_POOL_SIZE_OFFSET) as usize * mem::size_of::<u64>();
            ptr::copy_nonoverlapping(
                self.ptr.offset(BYTE_POOL_OFFSET) as *mut u8,
                buf.as_mut_ptr(),
                total_bytes,
            );
            Ok(total_bytes)
        }
    }
}

// NOTE(blt) -- all of this could be made much more compact, stuffing multiple
// signals into words and what not
impl Comm {
    pub fn new(ptr: *mut libc::c_void, len: usize) -> Self {
        assert!(::std::mem::size_of::<usize>() == 8);
        assert!(len > 128); // TODO(blt) -- make this a real bound
        Self {
            ptr: ptr as *mut u64,
            // len,
        }
    }

    //
    // server

    fn server_set_status(&mut self, stat: u64) -> () {
        unsafe {
            *self.ptr.offset(SERVER_STATUS_OFFSET) = stat;
        }
    }
    pub fn server_reset(&mut self) -> () {
        self.server_set_status(SERVER_DEFAULT);
    }
    pub fn server_ready(&mut self) -> () {
        self.server_set_status(SERVER_READY);
    }
    pub fn server_status(&self) -> ServerStatus {
        let status = unsafe { *self.ptr.offset(SERVER_STATUS_OFFSET) };
        match status {
            SERVER_DEFAULT => ServerStatus::Default,
            SERVER_READY => ServerStatus::Ready,
            _ => unreachable!(),
        }
    }

    //
    // client

    fn client_set_status(&mut self, stat: u64) -> () {
        unsafe {
            *self.ptr.offset(CLIENT_STATUS_OFFSET) = stat;
        }
    }
    pub fn client_reset(&mut self) -> () {
        self.client_set_status(CLIENT_DEFAULT);
    }
    pub fn client_ready(&mut self) -> () {
        self.client_set_status(CLIENT_READY);
    }
    pub fn client_test_status(&mut self, status: TestStatus) -> () {
        let s = match status {
            TestStatus::Passed => CLIENT_TEST_PASSED,
            TestStatus::Skipped => CLIENT_TEST_SKIPPED,
            TestStatus::Failed => CLIENT_TEST_FAILED,
            TestStatus::InsufficientBytes => CLIENT_TEST_INSUFFICIENT_BYTES,
        };
        self.client_set_status(s);
    }
    pub fn client_status(&self) -> ClientStatus {
        let status = unsafe { *self.ptr.offset(CLIENT_STATUS_OFFSET) };
        match status {
            CLIENT_DEFAULT => ClientStatus::Default,
            CLIENT_READY => ClientStatus::Ready,
            CLIENT_TEST_PASSED => ClientStatus::Test(TestStatus::Passed),
            CLIENT_TEST_SKIPPED => ClientStatus::Test(TestStatus::Skipped),
            CLIENT_TEST_FAILED => ClientStatus::Test(TestStatus::Failed),
            CLIENT_TEST_INSUFFICIENT_BYTES => ClientStatus::Test(TestStatus::InsufficientBytes),
            _ => unreachable!(),
        }
    }
}
