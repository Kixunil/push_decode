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
