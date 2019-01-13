use std::{io, mem, ptr};

pub struct Comm {
    ptr: *mut u64,
    len: usize,
}

const SERVER_READY_OFFSET: isize = 0;
const CLIENT_READY_OFFSET: isize = 1;
const PASSED_TESTS_OFFSET: isize = 2;
const SKIPPED_TESTS_OFFSET: isize = 3;
const FAILED_TESTS_OFFSET: isize = 4;
const BYTE_POOL_SIZE_OFFSET: isize = 5;
const BYTE_POOL_OFFSET: isize = 6;

pub enum Stat {
    SkippedTests,
    PassedTests,
    FailedTests,
}

impl io::Write for Comm {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let total_bytes = buf.len();
        // TODO(blt) -- check that the buffer being written isn't too big for the space we have
        unsafe {
            println!("WRITING TOTAL_BYTES: {}", total_bytes);
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
            println!(
                "READING TOTAL_BYTES: {}",
                *self.ptr.offset(BYTE_POOL_SIZE_OFFSET)
            );
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
            len,
        }
    }

    pub fn stat(&mut self, stat: Stat) -> u64 {
        let offset = match stat {
            Stat::SkippedTests => SKIPPED_TESTS_OFFSET,
            Stat::PassedTests => PASSED_TESTS_OFFSET,
            Stat::FailedTests => FAILED_TESTS_OFFSET,
        };
        unsafe { *self.ptr.offset(offset) }
    }
    pub fn incr(&mut self, stat: Stat) -> () {
        let offset = match stat {
            Stat::SkippedTests => SKIPPED_TESTS_OFFSET,
            Stat::PassedTests => PASSED_TESTS_OFFSET,
            Stat::FailedTests => FAILED_TESTS_OFFSET,
        };
        unsafe {
            *self.ptr.offset(offset) += 1;
        }
    }

    pub fn server_ready(&mut self) -> () {
        unsafe {
            *self.ptr.offset(SERVER_READY_OFFSET) = 1;
        }
    }
    pub fn is_server_ready(&mut self) -> bool {
        unsafe { *self.ptr.offset(SERVER_READY_OFFSET) == 1 }
    }

    pub fn client_ready(&mut self) -> () {
        unsafe {
            *self.ptr.offset(CLIENT_READY_OFFSET) = 1;
        }
    }
    pub fn client_unready(&mut self) -> () {
        unsafe {
            *self.ptr.offset(CLIENT_READY_OFFSET) = 0;
        }
    }
    pub fn is_client_ready(&mut self) -> bool {
        unsafe { *self.ptr.offset(CLIENT_READY_OFFSET) == 1 }
    }
}
