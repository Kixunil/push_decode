use alloc::vec::Vec;

use crate::Decoder;
use crate::error::UnexpectedEnd;

pub struct ByteVecDecoder {
    buf: Vec<u8>,
    required: usize,
}

impl ByteVecDecoder {
    pub fn new(required_bytes: usize) -> Self {
        ByteVecDecoder {
            buf: Vec::with_capacity(required_bytes),
            required: required_bytes,
        }
    }
}

impl Decoder for ByteVecDecoder {
    type Value = Vec<u8>;
    type Error = UnexpectedEnd;

    fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let to_copy = bytes.len().min(self.required - self.buf.len());
        self.buf.extend_from_slice(&bytes[..to_copy]);
        Ok(to_copy)
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        if self.buf.len() < self.required {
            Err(UnexpectedEnd { missing: self.required - self.buf.len() })
        } else {
            Ok(self.buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ByteVecDecoder;
    use crate::Decoder;

    #[test]
    fn empty() {
        let mut decoder = ByteVecDecoder::new(0);
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 0);
        decoder.end().unwrap();
    }

    #[test]
    fn empty_immediate_end() {
        let decoder = ByteVecDecoder::new(0);
        decoder.end().unwrap();
    }

    #[test]
    fn one() {
        let mut decoder = ByteVecDecoder::new(1);
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn one_immediate_end() {
        let mut decoder = ByteVecDecoder::new(1);
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn two_single() {
        let mut decoder = ByteVecDecoder::new(2);
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split() {
        let mut decoder = ByteVecDecoder::new(2);
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split_extra() {
        let mut decoder = ByteVecDecoder::new(2);
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21, 47]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }
}
