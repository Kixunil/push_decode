use crate::Encoder;

/// Trivially directly encodes byte slices.
///
/// This accepts any byte-carrying type, notably including `&str`/`String`.
#[must_use = "encoders do nothing by themselves"]
pub struct BytesEncoder<T: AsRef<[u8]>>(T);

impl<T: AsRef<[u8]>> BytesEncoder<T> {
    /// Creates the encoder.
    pub fn new(bytes: T) -> Self {
        BytesEncoder(bytes)
    }
}

impl<T: AsRef<[u8]>> Encoder for BytesEncoder<T> {
    fn encoded_chunk(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn next(&mut self) -> bool {
        false
    }
}

impl<T> From<T> for BytesEncoder<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Encoder;

    #[test]
    fn empty() {
        use crate::encoders::BytesEncoder;
        let encoder = BytesEncoder::new([]);
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, []);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn one() {
        use crate::encoders::BytesEncoder;
        let encoder = BytesEncoder::new([42]);
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, [42]);
    }
}
