use core::fmt;

#[derive(Debug, Clone)]
pub struct UnexpectedEnd {
    pub(crate) missing: usize,
}

impl fmt::Display for UnexpectedEnd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let plural = match self.missing {
            1 => " was",
            _ => "s were",
        };
        write!(f, "end of stream reached too soon, {} more byte{} required", self.missing, plural)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UnexpectedEnd {}

#[derive(Debug, Clone)]
pub struct BufferOverflow {
    pub(crate) bytes_past_end: usize,
}

impl fmt::Display for BufferOverflow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "attempted to write {} bytes past the end of the buffer", self.bytes_past_end)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BufferOverflow {}
