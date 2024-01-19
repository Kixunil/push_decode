use crate::Decoder;

#[derive(Debug)]
pub struct ThenTry<E, First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Result<Second, E>>(ThenTryState<E, First, Second, Fun>) where E: From<First::Error> + From<Second::Error>;

impl<E, First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Result<Second, E>> ThenTry<E, First, Second, Fun> where E: From<First::Error> + From<Second::Error> {
    pub(crate) fn new(first: First, fun: Fun) -> Self {
        ThenTry(ThenTryState::First(first, fun))
    }
}

#[derive(Debug)]
enum ThenTryState<E, First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Result<Second, E>> where E: From<First::Error> + From<Second::Error> {
    First(First, Fun),
    Second(Second),
    Panicked,
    // improves debugging a bit
    Errored,
}

macro_rules! try_and_mark {
    ($result:expr, $state:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                $state.0 = ThenTryState::Errored;
                return Err(error.into())
            }
        }
    }
}

impl<E, First: Decoder, Second: Decoder, Fun: FnOnce(First::Value) -> Result<Second, E>> Decoder for ThenTry<E, First, Second, Fun> where E: From<First::Error>, E: From<Second::Error> {
    type Value = Second::Value;
    type Error = E;

    #[inline]
    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        let decoder = core::mem::replace(&mut self.0, ThenTryState::Panicked);
        match decoder {
            ThenTryState::First(mut first, fun) => {
                try_and_mark!(first.decode_chunk(bytes), self);
                if !bytes.is_empty() {
                    let val = try_and_mark!(first.end(), self);
                    let mut second = try_and_mark!(fun(val), self);
                    try_and_mark!(second.decode_chunk(bytes), self);
                    self.0 = ThenTryState::Second(second);
                    Ok(())
                } else {
                    self.0 = ThenTryState::First(first, fun);
                    Ok(())
                }
            },
            ThenTryState::Second(mut second) => {
                try_and_mark!(second.decode_chunk(bytes), self);
                self.0 = ThenTryState::Second(second);
                Ok(())
            },
            ThenTryState::Panicked => panic!("Decoder::decode_chunk called after it already panicked"),
            ThenTryState::Errored => panic!("Decoder::decode_chunk called after it already returned an error"),
        }
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        match self.0 {
            ThenTryState::First(first, fun) => {
                // This is strange but allows for empty decoders if anyone ever needs them
                let val = first.end()?;
                fun(val)?.end().map_err(Into::into)
            },
            ThenTryState::Second(second) => {
                second.end().map_err(Into::into)
            },
            ThenTryState::Panicked => panic!("Decoder::end called after Decoder::decode_chunk already panicked"),
            ThenTryState::Errored => panic!("Decoder::end called after Decoder::decode_chunk already returned an error"),
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
