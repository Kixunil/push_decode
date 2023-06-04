use core::fmt;

#[derive(Debug, Clone)]
pub struct UnexpectedEnd {
    pub(crate) missing: usize,
}

impl fmt::Display for UnexpectedEnd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "end of stream reached too soon, more {} bytes were required", self.missing)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UnexpectedEnd {}
