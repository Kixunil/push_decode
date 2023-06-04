use crate::Encoder;

#[derive(Debug, Clone)]
pub struct Chain<A, B> {
    // Not an enum to avoid moving stuff around when going from one to the other.
    first: Option<A>,
    second: B,
}

impl<A: Encoder, B: Encoder> Chain<A, B> {
    pub(crate) fn new(first: A, second: B) -> Self {
        let first = if first.encoded_chunk().is_empty() {
            None
        } else {
            Some(first)
        };
        Chain {
            first,
            second,
        }
    }
}

impl<A: Encoder, B: Encoder> Encoder for Chain<A, B> {
    fn encoded_chunk(&self) -> &[u8] {
        match &self.first {
            Some(first) => first.encoded_chunk(),
            None => self.second.encoded_chunk(),
        }
    }

    fn next(&mut self) -> bool {
        match &mut self.first {
            Some(first) => {
                if first.next() {
                    true
                } else {
                    self.first = None;
                    !self.second.encoded_chunk().is_empty()
                }
            },
            None => self.second.next(),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Encoder;

    #[test]
    fn chain() {
        use crate::encoders::BytesEncoder;
        let encoder = BytesEncoder::new([1]).chain(BytesEncoder::new([2]));
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, [1, 2]);
    }
}
