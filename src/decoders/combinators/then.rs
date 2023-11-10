use either::Either;
use crate::Decoder;

#[derive(Debug)]
pub struct Then<First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Second>(ThenState<First, Second, Fun>);

impl<First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Second> Then<First, Second, Fun> {
    pub(crate) fn new(first: First, fun: Fun) -> Self {
        Then(ThenState::First(first, fun))
    }
}

#[derive(Debug)]
enum ThenState<First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Second> {
    First(First, Fun),
    Second(Second),
    Panicked,
    // improves debugging a bit
    Errored,
}

impl<First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Second> Decoder for Then<First, Second, Fun> {
    type Value = Second::Value;
    type Error = Either<First::Error, Second::Error>;

    #[inline]
    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        let decoder = core::mem::replace(&mut self.0, ThenState::Panicked);
        match decoder {
            ThenState::First(mut first, fun) => {
                first.decode_chunk(bytes).map_err(Either::Left)?;
                if !bytes.is_empty() {
                    let result = first.end();
                    self.0 = ThenState::Errored;
                    let val = result.map_err(Either::Left)?;
                    self.0 = ThenState::Panicked;
                    let mut second = fun(val);
                    let result = second.decode_chunk(bytes);
                    self.0 = ThenState::Second(second);
                    result.map_err(Either::Right)
                } else {
                    self.0 = ThenState::First(first, fun);
                    Ok(())
                }
            },
            ThenState::Second(mut second) => {
                let result = second.decode_chunk(bytes);
                self.0 = ThenState::Second(second);
                result.map_err(Either::Right)
            },
            ThenState::Panicked => panic!("Decoder::decode_chunk called after it already panicked"),
            ThenState::Errored => panic!("Decoder::decode_chunk called after it already returned an error"),
        }
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        match self.0 {
            ThenState::First(first, fun) => {
                // This is strange but allows for empty decoders if anyone ever needs them
                let val = first.end().map_err(Either::Left)?;
                fun(val).end().map_err(Either::Right)
            },
            ThenState::Second(second) => {
                second.end().map_err(Either::Right)
            },
            ThenState::Panicked => panic!("Decoder::end called after Decoder::decode_chunk already panicked"),
            ThenState::Errored => panic!("Decoder::end called after Decoder::decode_chunk already returned an error"),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Decoder;
    use crate::decoders::ByteVecDecoder;
    use crate::decoders::U8Decoder;

    #[test]
    fn chain_all() {
        let mut decoder = U8Decoder::new().then(|len| ByteVecDecoder::new(len.into()));
        assert_eq!(decoder.bytes_received(&[2, 3, 4]).unwrap(), 3);
        assert_eq!(decoder.end().unwrap(), vec![3, 4]);
    }

    #[test]
    fn chain_extra() {
        let mut decoder = U8Decoder::new().then(|len| ByteVecDecoder::new(len.into()));
        assert_eq!(decoder.bytes_received(&[2, 3, 4, 5]).unwrap(), 3);
        assert_eq!(decoder.end().unwrap(), vec![3, 4]);
    }

    #[test]
    fn chain_split() {
        let mut decoder = U8Decoder::new().then(|len| ByteVecDecoder::new(len.into()));
        assert_eq!(decoder.bytes_received(&[2]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[3, 4]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), vec![3, 4]);
    }

    #[test]
    fn chain_split_extra() {
        let mut decoder = U8Decoder::new().then(|len| ByteVecDecoder::new(len.into()));
        assert_eq!(decoder.bytes_received(&[2]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[3, 4, 5]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), vec![3, 4]);
    }
}
