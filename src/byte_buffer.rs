use crate::arbitrary::Unstructured;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// An enumeration of buffer creation errors
#[derive(Debug, Clone, Copy)]
pub enum BufferInitError {
    /// The input buffer is empty, causing construction of some buffer types to
    /// fail
    EmptyInput,
    /// The input buffer is too small, please increase size
    BufferTooSmall,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Potential errors of the [`ByteBuffer`]
pub enum BufferOpError {
    /// A request was made to fill a buffer, shift etc but there were
    /// insufficient bytes to accomplish this
    InsufficientBytes,
    /// The requested shift failed because this shift would cause wrap-around,
    /// duplicating tests
    ShiftWrapAround,
}

/// A source of unstructured data which returns the same data over and over again
///
/// This buffer acts as a byte buffer over the source of unstructured data,
/// allowing for an infinite amount of not-very-random data.
pub struct ByteBuffer {
    buffer: Vec<u8>,
    rng: SmallRng,
    pub offset: usize,
    shift_offset: usize,
    virtual_len: usize,
}

impl ByteBuffer {
    /// Create a new ByteBuffer
    pub fn new(capacity: usize, seed: u64) -> Result<Self, BufferInitError> {
        if capacity == 0 {
            return Err(BufferInitError::EmptyInput);
        }
        if capacity <= 2 {
            return Err(BufferInitError::BufferTooSmall);
        }
        let mut buffer: Vec<u8> = Vec::with_capacity(capacity);
        let mut rng = SmallRng::seed_from_u64(seed);
        for _ in 0..capacity {
            buffer.push(rng.gen::<u8>())
        }
        Ok(ByteBuffer {
            virtual_len: buffer.len() / 1,
            shift_offset: 0,
            offset: 0,
            buffer,
            rng,
        })
    }

    pub fn shift_right(&mut self) -> Result<(), BufferOpError> {
        if self.shift_offset != 0 && ((self.shift_offset + 1) % self.buffer.len() == 0) {
            return Err(BufferOpError::ShiftWrapAround);
        } else {
            self.shift_offset += 1;
            self.offset = self.shift_offset;
            return Ok(());
        }
    }

    pub fn soft_reset(&mut self) {
        self.offset = self.shift_offset;
        self.virtual_len = self.buffer.len() / 2;
    }

    pub fn hard_reset(&mut self) {
        self.soft_reset();
        for b in self.buffer.iter_mut() {
            *b = self.rng.gen::<u8>();
        }
    }

    pub fn shrink_from(&mut self, offset: usize) -> usize {
        self.offset = offset;
        self.virtual_len /= 2;
        self.virtual_len
    }
}

impl Unstructured for ByteBuffer {
    type Error = BufferOpError;
    fn fill_buffer(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let b = [
            &self.buffer[self.offset..self.virtual_len],
            &self.buffer[..self.offset],
        ];
        let it = ::std::iter::repeat(&b[..]).flat_map(|x| x).flat_map(|&x| x);
        self.offset = (self.offset + buffer.len()) % self.virtual_len;
        for (d, f) in buffer.iter_mut().zip(it) {
            *d = *f;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn byte_buffer_fill_buffer() {
        let x = [1, 2, 3, 4];
        let mut rb = ByteBuffer::new(&x).unwrap();
        let mut z = [0; 10];
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [1, 2, 3, 4, 1, 2, 3, 4, 1, 2]);
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [3, 4, 1, 2, 3, 4, 1, 2, 3, 4]);
    }

    #[test]
    fn byte_buffer_fill_buffer_shrink() {
        let x = [1, 2, 3, 4];
        let mut rb = ByteBuffer::new(&x).unwrap();
        let mut z = [0; 10];
        assert_eq!(2, rb.shrink());
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [1, 2, 1, 2, 1, 2, 1, 2, 1, 2]);
        assert_eq!(1, rb.shrink());
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn byte_buffer_fill_buffer_shift() {
        let x = [1, 2, 3, 4];
        let mut rb = ByteBuffer::new(&x).unwrap();
        let mut z = [0; 10];
        rb.shift_right(1).unwrap();
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [2, 3, 4, 1, 2, 3, 4, 1, 2, 3]);
        rb.shift_right(1).unwrap();
        rb.fill_buffer(&mut z).unwrap();
        assert_eq!(z, [1, 2, 3, 4, 1, 2, 3, 4, 1, 2]);
    }

    #[test]
    fn byte_buffer_container_size() {
        let x = [1, 2, 3, 4, 5];
        let mut rb = ByteBuffer::new(&x).unwrap().container_size_limit(11);
        assert_eq!(rb.container_size().unwrap(), 9);
        assert_eq!(rb.container_size().unwrap(), 1);
        assert_eq!(rb.container_size().unwrap(), 2);
        assert_eq!(rb.container_size().unwrap(), 6);
        assert_eq!(rb.container_size().unwrap(), 1);
    }
}
