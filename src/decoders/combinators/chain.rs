use core::fmt;
use either::Either;
use crate::Decoder;

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
    fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        match &mut self.0 {
            State::First(first, _) => {
                let len = first.bytes_received(bytes).map_err(Either::Left)?;
                if len < bytes.len() {
                    let (first, second) = self.0.take_first();
                    let first_val = first.end()
                        .map_err(|error| { self.0 = State::Errored; Either::Left(error)})?;
                    self.0 = State::Second(first_val, second);
                    self.bytes_received(&bytes[len..]).map(|len2| len2 + len)
                } else {
                    Ok(len)
                }
            },
            State::Second(_, second) => {
                second.bytes_received(bytes).map_err(Either::Right)
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
