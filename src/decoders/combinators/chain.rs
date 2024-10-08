use core::fmt;
use either::Either;
use crate::{Decoder, KnownMinLenDecoder};

pub struct Chain<First: Decoder, Second: Decoder>(State<First, Second>);

impl<First: Decoder, Second: Decoder> Chain<First, Second> {
    pub(crate) fn new(first: First, second: Second) -> Self {
        Chain(State::First(first, second))
    }
}

impl <First, Second> fmt::Debug for Chain<First, Second>
where First: Decoder + fmt::Debug,
      Second: Decoder + fmt::Debug,
      First::Value: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            State::First(a, b) => f.debug_tuple("Chain::First").field(a).field(b).finish(),
            State::Second(a, b) => f.debug_tuple("Chain::Second").field(a).field(b).finish(),
            State::Errored => f.debug_tuple("Chain::Errored").finish(),
            State::Panicked => f.debug_tuple("Chain::Panicked").finish(),
        }
    }
}

enum State<A: Decoder, B: Decoder> {
    First(A, B),
    Second(A::Value, B),
    Errored,
    Panicked
}

impl<A: Decoder, B: Decoder> State<A, B> {
    #[track_caller]
    fn take_first(&mut self) -> (A, B) {
        match core::mem::replace(self, State::Panicked) {
            State::First(a, b) => (a, b),
            _ => panic!("invalid state"),
        }
    }
}

impl<First: Decoder, Second: Decoder> Decoder for Chain<First, Second> {
    type Value = (First::Value, Second::Value);
    type Error = Either<First::Error, Second::Error>;

    #[inline]
    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        match &mut self.0 {
            State::First(first, _) => {
                first.decode_chunk(bytes).map_err(Either::Left)?;
                if !bytes.is_empty() {
                    let (first, second) = self.0.take_first();
                    let first_val = first.end()
                        .map_err(|error| { self.0 = State::Errored; Either::Left(error)})?;
                    self.0 = State::Second(first_val, second);
                    self.decode_chunk(bytes)
                } else {
                    Ok(())
                }
            },
            State::Second(_, second) => {
                second.decode_chunk(bytes).map_err(Either::Right)
            },
            State::Errored => panic!("use of failed decoder"),
            State::Panicked => panic!("use of panicked decoder"),
        }
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        match self.0 {
            State::First(first, second) => {
                let first = first.end().map_err(Either::Left)?;
                let second = second.end().map_err(Either::Right)?;
                Ok((first, second))
            },
            State::Second(first, second) => {
                let second = second.end().map_err(Either::Right)?;
                Ok((first, second))
            },
            State::Errored => panic!("use of failed decoder"),
            State::Panicked => panic!("use of panicked decoder"),
        }
    }
}

impl<First: KnownMinLenDecoder, Second: KnownMinLenDecoder> KnownMinLenDecoder for Chain<First, Second> {
    fn min_required_bytes(&self) -> usize {
        match &self.0 {
            State::First(first, second) => {
                first.min_required_bytes().saturating_add(second.min_required_bytes())
            },
            State::Second(_, second) => {
                second.min_required_bytes()
            },
            State::Errored => panic!("use of failed decoder"),
            State::Panicked => panic!("use of panicked decoder"),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Decoder;
    use crate::decoders::ByteVecDecoder;

    #[test]
    fn chain_all() {
        let mut decoder = ByteVecDecoder::new(1).chain(ByteVecDecoder::new(2));
        assert_eq!(decoder.bytes_received(&[1, 2, 3]).unwrap(), 3);
        assert_eq!(decoder.end().unwrap(), (vec![1], vec![2, 3]));
    }

    #[test]
    fn chain_extra() {
        let mut decoder = ByteVecDecoder::new(1).chain(ByteVecDecoder::new(2));
        assert_eq!(decoder.bytes_received(&[1, 2, 3, 4]).unwrap(), 3);
        assert_eq!(decoder.end().unwrap(), (vec![1], vec![2, 3]));
    }

    #[test]
    fn chain_split() {
        let mut decoder = ByteVecDecoder::new(1).chain(ByteVecDecoder::new(2));
        assert_eq!(decoder.bytes_received(&[1]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[2, 3]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), (vec![1], vec![2, 3]));
    }

    #[test]
    fn chain_split_extra() {
        let mut decoder = ByteVecDecoder::new(1).chain(ByteVecDecoder::new(2));
        assert_eq!(decoder.bytes_received(&[1]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[2, 3, 4]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), (vec![1], vec![2, 3]));
    }
}
