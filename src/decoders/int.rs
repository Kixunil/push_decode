use core::marker::PhantomData;
use crate::Decoder;
use crate::error::UnexpectedEnd;
use crate::int::*;

pub struct IntDecoder<T: Int, Endian: ByteOrder>(T::Decoder, PhantomData<fn() -> T>, PhantomData<Endian>);

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

impl<T: Int> Decoder for IntDecoder<T, BigEndian> {
    type Value = T;
    type Error = UnexpectedEnd;

    fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.0.bytes_received(bytes)
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        self.0.end().map(Int::from_be_bytes)
    }
}

impl<T: Int> Decoder for IntDecoder<T, LittleEndian> {
    type Value = T;
    type Error = UnexpectedEnd;

    fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.0.bytes_received(bytes)
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        self.0.end().map(Int::from_le_bytes)
    }
}
