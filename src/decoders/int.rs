use core::fmt;
use core::marker::PhantomData;
use crate::{Decoder, KnownMinLenDecoder};
use crate::error::UnexpectedEnd;
use crate::int::*;

pub struct IntDecoder<T: Int, Endian: ByteOrder>(T::InnerDecoder, PhantomData<fn() -> T>, PhantomData<Endian>);

impl<T: Int, Endian: ByteOrder> IntDecoder<T, Endian> {
    pub fn new() -> Self {
        IntDecoder(Default::default(), Default::default(), Default::default())
    }
}

impl<T: Int, Endian: ByteOrder> Default for IntDecoder<T, Endian> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Int, Endian: ByteOrder> fmt::Debug for IntDecoder<T, Endian> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "IntDecoder<{}>({:?})", core::any::type_name::<Endian>(), self.0)
    }
}

impl<T: Int> Decoder for IntDecoder<T, BigEndian> {
    type Value = T;
    type Error = UnexpectedEnd;

    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        self.0.decode_chunk(bytes)
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        self.0.end().map(Int::from_be_bytes)
    }
}

impl<T: Int> Decoder for IntDecoder<T, LittleEndian> {
    type Value = T;
    type Error = UnexpectedEnd;

    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        self.0.decode_chunk(bytes)
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        self.0.end().map(Int::from_le_bytes)
    }
}

impl<T: Int> KnownMinLenDecoder for IntDecoder<T, BigEndian> {
    fn min_required_bytes(&self) -> usize {
        self.0.min_required_bytes()
    }
}
