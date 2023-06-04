use core::marker::PhantomData;
use crate::int::*;
use crate::Encoder;

/// Encodes an integer as an array of bytes.
pub struct IntEncoder<T: Int>(T::Bytes, PhantomData<fn(T)>);

impl<T: Int> IntEncoder<T> {
    /// Creates big-endian encoder.
    pub fn new_be(value: T) -> Self {
        IntEncoder(value.to_be_bytes(), Default::default())
    }

    /// Creates little-endian encoder.
    pub fn new_le(value: T) -> Self {
        IntEncoder(value.to_le_bytes(), Default::default())
    }
}

impl<T: Int> Encoder for IntEncoder<T> {
    fn encoded_chunk(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn next(&mut self) -> bool {
        false
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Encoder;

    #[test]
    fn be() {
        let encoder = super::IntEncoder::new_be(1u32);
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, [0, 0, 0, 1]);
    }

    #[test]
    fn le() {
        let encoder = super::IntEncoder::new_le(1u32);
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, [1, 0, 0, 0]);
    }
}
