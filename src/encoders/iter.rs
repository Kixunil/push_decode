use crate::Encoder;

/// Encodes an iterator of items that can be encoded.
///
/// The items are encoded one after the other with no separators.
pub struct IterEncoder<I: Iterator, E = <I as Iterator>::Item>(EncoderState<I, E>) where I::Item: Into<E>, E: Encoder;

impl<I: Iterator, E: Encoder> IterEncoder<I, E> where I::Item: Into<E>, E: Encoder {
    pub fn new(iter: impl IntoIterator<IntoIter=I>) -> Self where I::Item: Into<E> {
        let mut iter = iter.into_iter().fuse();
        // Empty elements must be skipped
        let state = loop {
            match iter.next() {
                Some(first) => {
                    let encoder = first.into();
                    if !encoder.encoded_chunk().is_empty() {
                        break EncoderState::Encoding { current: encoder, remaining: iter };
                    }
                },
                None => break EncoderState::Done,
            }
        };
        Self(state)
    }
}

impl<T: IntoIterator<IntoIter=I>, I: Iterator, E: Encoder> From<T> for IterEncoder<I, E> where I::Item: Into<E> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

enum EncoderState<I: Iterator, T: Encoder> where I::Item: Into<T> {
    Encoding { current: T, remaining: core::iter::Fuse<I> },
    Done,
}

impl<I: Iterator, T: Encoder + From<I::Item>> Encoder for IterEncoder<I, T> {
    fn encoded_chunk(&self) -> &[u8] {
        match &self.0 {
            EncoderState::Encoding { current, .. } => current.encoded_chunk(),
            EncoderState::Done => &[],
        }
    }

    fn next(&mut self) -> bool {
        match &mut self.0 {
            EncoderState::Encoding { current, remaining } => {
                if current.next() {
                    true
                } else {
                    loop {
                        match remaining.next() {
                            Some(next) => {
                                *current = next.into();
                                if !current.encoded_chunk().is_empty() {
                                    break true;
                                }
                            },
                            None => {
                                self.0 = EncoderState::Done;
                                break false;
                            }
                        }
                    }
                }
            },
            EncoderState::Done => false,
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use super::super::BytesEncoder;

    #[track_caller]
    fn expect(encoder: impl Encoder, expected: &[u8]) {
        let mut vec = alloc::vec::Vec::new();
        encoder.write_to_vec(&mut vec);
        assert_eq!(vec, expected);
    }

    #[test]
    fn empty_iter() {
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(core::iter::empty::<BytesEncoder<[u8; 1]>>());
        expect(encoder, &[]);
    }

    #[test]
    fn iter_of_empty_arrays() {
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(core::iter::once(BytesEncoder::new([0u8; 0])));
        expect(encoder, &[]);
    }

    #[test]
    fn one() {
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(core::iter::once(BytesEncoder::single_byte(1)));
        expect(encoder, &[1]);
    }

    #[test]
    fn two_iterations() {
        let items = [[1; 1], [2; 1]];
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(items);
        expect(encoder, &[1, 2]);
    }

    #[test]
    fn two_empty_one_nonempty() {
        let items = [&[] as &[u8], &[], &[1]];
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(items);
        expect(encoder, &[1]);
    }

    #[test]
    fn one_nonempty_two_empty_one_non_empty() {
        let items = [&[1u8] as &[_], &[], &[], &[2]];
        let encoder = IterEncoder::<_, BytesEncoder<_>>::new(items);
        expect(encoder, &[1, 2]);
    }
}
