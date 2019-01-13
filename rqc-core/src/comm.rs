pub struct Comm {
    ptr: *mut u64,
    len: usize,
}

const SERVER_READY_OFFSET: isize = 0;
const CLIENT_READY_OFFSET: isize = 1;

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
