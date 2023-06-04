use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use crate::Decoder;
use crate::error::UnexpectedEnd;

pub struct Utf8StringDecoder {
    // invariant: buf[..valid_up_to] is a valid string
    buf: Vec<u8>,
    valid_up_to: usize,
    required: usize,
}

impl Utf8StringDecoder {
    pub fn new(len_bytes: usize) -> Self {
        Utf8StringDecoder {
            buf: Vec::new(),
            valid_up_to: 0,
            required: len_bytes,
        }
    }
}

impl Decoder for Utf8StringDecoder {
    type Value = String;
    type Error = Error;

    fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let to_copy = bytes.len().min(self.required - self.buf.len());
        if to_copy == 0 {
            return Ok(0);
        }
        if self.buf.capacity() == 0 {
            self.buf.reserve(self.required);
        }
        // pre-check to avoid copying if the bytes are invalid anyway
        if self.valid_up_to == self.buf.len() {
            match core::str::from_utf8(&bytes[..to_copy]) {
                Ok(_) => {
                    self.buf.extend_from_slice(&bytes[..to_copy]);
                    self.valid_up_to += to_copy;
                    Ok(to_copy)
                },
                Err(error) if error.error_len().is_none() => {
                    self.buf.extend_from_slice(&bytes[..to_copy]);
                    self.valid_up_to += error.valid_up_to();
                    Ok(to_copy)
                },
                Err(error) => Err(Error::InvalidUtf8(error)),
            }
        } else {
            self.buf.extend_from_slice(&bytes[..to_copy]);
            match core::str::from_utf8(&self.buf[self.valid_up_to..]) {
                Ok(_) => {
                    self.valid_up_to = self.buf.len();
                    Ok(to_copy)
                },
                Err(error) if error.error_len().is_none() => {
                    self.valid_up_to += error.valid_up_to();
                    Ok(to_copy)
                },
                Err(error) => Err(Error::InvalidUtf8(error)),
            }
        }
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        if self.buf.len() < self.required {
            Err(Error::UnexpectedEnd(UnexpectedEnd { missing: self.required - self.buf.len() }))
        } else {
            if self.valid_up_to == self.buf.len() {
                // SAFETY: guaranteed by invariant and the check above
                Ok(unsafe { String::from_utf8_unchecked(self.buf) })
            } else {
                // Unfortunately we have to re-validate to produce the error. :(
                Err(Error::InvalidUtf8(core::str::from_utf8(&self.buf).unwrap_err()))
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidUtf8(core::str::Utf8Error),
    UnexpectedEnd(UnexpectedEnd),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidUtf8(_) => write!(f, "supplied bytes are not valid UTF-8"),
            Error::UnexpectedEnd(_) => write!(f, "unexpected end"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidUtf8(error) => Some(error),
            Error::UnexpectedEnd(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Utf8StringDecoder;
    use crate::Decoder;

    #[test]
    fn empty() {
        let mut decoder = Utf8StringDecoder::new(0);
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 0);
        assert_eq!(decoder.end().unwrap(), "");
    }

    #[test]
    fn empty_immediate_end() {
        let decoder = Utf8StringDecoder::new(0);
        assert_eq!(decoder.end().unwrap(), "");
    }

    #[test]
    fn one() {
        let mut decoder = Utf8StringDecoder::new(1);
        assert_eq!(decoder.bytes_received(b"xy").unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), "x");
    }

    #[test]
    fn one_immediate_end() {
        let mut decoder = Utf8StringDecoder::new(1);
        assert_eq!(decoder.bytes_received(b"x").unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), "x");
    }

    #[test]
    fn two_single() {
        let mut decoder = Utf8StringDecoder::new(2);
        assert_eq!(decoder.bytes_received(b"xy").unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), "xy");
    }

    #[test]
    fn two_split() {
        let mut decoder = Utf8StringDecoder::new(2);
        assert_eq!(decoder.bytes_received(b"x").unwrap(), 1);
        assert_eq!(decoder.bytes_received(b"y").unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), "xy");
    }

    #[test]
    fn two_split_extra() {
        let mut decoder = Utf8StringDecoder::new(2);
        assert_eq!(decoder.bytes_received(b"x").unwrap(), 1);
        assert_eq!(decoder.bytes_received(b"yz").unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), "xy");
    }

    #[test]
    fn unicode_split() {
        let mut decoder = Utf8StringDecoder::new(4);
        assert_eq!(decoder.bytes_received(&[0xF0, 0x9F]).unwrap(), 2);
        assert_eq!(decoder.bytes_received(&[0xA6, 0x80]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), "🦀");
    }

    #[test]
    fn broken_utf8() {
        let mut decoder = Utf8StringDecoder::new(3);
        assert_eq!(decoder.bytes_received(&[0xF0, 0x9F]).unwrap(), 2);
        assert_eq!(decoder.bytes_received(&[0xA6]).unwrap(), 1);
        decoder.end().unwrap_err();
    }
}
