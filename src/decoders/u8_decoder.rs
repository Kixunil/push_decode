use crate::{Decoder, KnownMinLenDecoder};
use crate::error::UnexpectedEnd;

#[derive(Default, Debug)]
pub struct U8Decoder {
    buf: Option<u8>,
}

impl U8Decoder {
    pub fn new() -> Self {
        U8Decoder {
            buf: None,
        }
    }
}

impl Decoder for U8Decoder {
    type Value = u8;
    type Error = UnexpectedEnd;

    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        match (self.buf, bytes.get(0)) {
            (None, Some(byte)) => {
                self.buf = Some(*byte);
                *bytes = &bytes[1..];
                Ok(())
            },
            _ => Ok(()),
        }
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        self.buf.ok_or(UnexpectedEnd { missing: 1 })
    }
}

impl KnownMinLenDecoder for U8Decoder {
    fn min_required_bytes(&self) -> usize {
        match &self.buf {
            None => 1,
            Some(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::U8Decoder;
    use crate::Decoder;

    #[test]
    fn empty() {
        let decoder = U8Decoder::new();
        decoder.end().unwrap_err();
    }

    #[test]
    fn one() {
        let mut decoder = U8Decoder::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), 42);
    }

    #[test]
    fn two() {
        let mut decoder = U8Decoder::new();
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), 42);
    }
}
