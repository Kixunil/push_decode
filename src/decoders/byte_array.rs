use crate::Decoder;
use crate::error::UnexpectedEnd;

#[derive(Debug)]
pub struct ByteArrayDecoder<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> ByteArrayDecoder<N> {
    pub fn new() -> Self {
        ByteArrayDecoder {
            buf: [0; N],
            len: 0,
        }
    }
}

impl<const N: usize> Default for ByteArrayDecoder<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Decoder for ByteArrayDecoder<N> {
    type Value = [u8; N];
    type Error = UnexpectedEnd;

    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        let to_copy = bytes.len().min(N - self.len);
        self.buf[self.len..(self.len + to_copy)].copy_from_slice(&bytes[..to_copy]);
        self.len += to_copy;
        *bytes = &bytes[to_copy..];
        Ok(())
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        if self.len < N {
            Err(UnexpectedEnd { missing: N - self.len })
        } else {
            Ok(self.buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ByteArrayDecoder;
    use crate::Decoder;

    #[test]
    fn empty() {
        let mut decoder = ByteArrayDecoder::<0>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 0);
        decoder.end().unwrap();
    }

    #[test]
    fn empty_immediate_end() {
        let decoder = ByteArrayDecoder::<0>::new();
        decoder.end().unwrap();
    }

    #[test]
    fn one() {
        let mut decoder = ByteArrayDecoder::<1>::new();
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn one_immediate_end() {
        let mut decoder = ByteArrayDecoder::<1>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn two_single() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split_extra() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21, 47]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }
}
