use crate::Encoder;

pub struct Then<A, B, F> {
    state: State<A, B, F>,
}

enum State<A, B, F> {
    First(A, F),
    Second(B),
}

impl<A: Encoder, B: Encoder, F: FnMut() -> B> Then<A, B, F> {
    pub(crate) fn new(first_encoder: A, second_encoder_constructor: F) -> Self {
        Then {
            state: State::First(first_encoder, second_encoder_constructor),
        }
    }
}

impl<A: Encoder, B: Encoder, F: FnMut() -> B> Encoder for Then<A, B, F> {
    #[track_caller]
    fn encoded_chunk(&self) -> &[u8] {
        match &self.state {
            State::First(encoder, _) => encoder.encoded_chunk(),
            State::Second(encoder) => encoder.encoded_chunk(),
        }
    }

    #[track_caller]
    fn next(&mut self) -> bool {
        match &mut self.state {
            State::First(encoder, fun) => {
                if encoder.next() {
                    true
                } else {
                    let new_encoder = fun();
                    let is_empty = new_encoder.encoded_chunk().is_empty();
                    self.state = State::Second(new_encoder);
                    !is_empty
                }
            },
            State::Second(encoder) => encoder.next(),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Encoder;

    #[test]
    fn then() {
        use crate::encoders::BytesEncoder;
        let encoder = BytesEncoder::new([1]).then(|| BytesEncoder::new([2]));
        let mut buf = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut buf);
        assert_eq!(buf, [1, 2]);
    }
}
