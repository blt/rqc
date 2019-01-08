use crate::arbitrary::{Arbitrary, Unstructured};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Potential errors of the [`ByteBuffer`]
pub enum BufferOpError {
    /// A request was made to fill a buffer, shift etc but there were
    /// insufficient bytes to accomplish this
    InsufficientBytes,
}

/// A source of unstructured data which returns the same data over and over again
///
/// This buffer acts as a byte buffer over the source of unstructured data,
/// allowing for an infinite amount of not-very-random data.
pub struct FiniteByteBuffer<'a> {
    buffer: &'a [u8],
    offset: usize,
    container_size_limit: usize,
}

impl<'a> FiniteByteBuffer<'a> {
    /// Create a new ByteBuffer
    pub fn new(buffer: &'a [u8]) -> Self {
        FiniteByteBuffer {
            offset: 0,
            buffer,
            container_size_limit: 256,
        }
    }

    /// Set the non-default container size limit
    pub fn container_size_limit(mut self, csl: usize) -> Self {
        self.container_size_limit = csl;
        self
    }
}

impl<'a> Unstructured for FiniteByteBuffer<'a> {
    type Error = BufferOpError;
    fn fill_buffer(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        if self.buffer.len().saturating_sub(self.offset) >= buffer.len() {
            let max = self.offset + buffer.len();
            for (i, idx) in (self.offset..max).enumerate() {
                buffer[i] = self.buffer[idx];
            }
            self.offset = max;
            Ok(())
        } else {
            Err(BufferOpError::InsufficientBytes)
        }
    }

    fn container_size(&mut self) -> Result<usize, Self::Error> {
        <usize as Arbitrary>::arbitrary(self).map(|x| x % self.container_size_limit)
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
