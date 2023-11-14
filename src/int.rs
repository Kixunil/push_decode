//! Tools for decoding and encoding integers.

use crate::{Decoder, decoders::ByteArrayDecoder};

pub trait Int: sealed::Int {
    #[doc(hidden)]
    type InnerDecoder: Decoder<Value = Self::Bytes, Error = crate::error::UnexpectedEnd> + Default + core::fmt::Debug;
    #[doc(hidden)]
    type Bytes: AsRef<[u8]>;

    #[doc(hidden)]
    fn from_le_bytes(bytes: Self::Bytes) -> Self;
    #[doc(hidden)]
    fn from_be_bytes(bytes: Self::Bytes) -> Self;
    #[doc(hidden)]
    fn to_le_bytes(self) -> Self::Bytes;
    #[doc(hidden)]
    fn to_be_bytes(self) -> Self::Bytes;
}

pub trait ByteOrder: sealed::ByteOrder {}

macro_rules! impl_int {
    ($($int:ty),+) => {
        $(
            impl Int for $int {
                type InnerDecoder = ByteArrayDecoder<{ core::mem::size_of::<Self>() }>;
                type Bytes = [u8; { core::mem::size_of::<Self>() }];

                fn from_le_bytes(bytes: Self::Bytes) -> Self {
                    <$int>::from_le_bytes(bytes)
                }

                fn from_be_bytes(bytes: Self::Bytes) -> Self {
                    <$int>::from_be_bytes(bytes)
                }

                fn to_le_bytes(self) -> Self::Bytes {
                    <$int>::to_le_bytes(self)
                }

                fn to_be_bytes(self) -> Self::Bytes {
                    <$int>::to_be_bytes(self)
                }
            }

            impl sealed::Int for $int {}
        )+
    }
}

impl_int!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128);

mod sealed {
    pub trait Int {}
    pub trait ByteOrder {}
}

pub struct BigEndian {}
pub struct LittleEndian {}

impl ByteOrder for BigEndian {}
impl sealed::ByteOrder for BigEndian {}
impl ByteOrder for LittleEndian {}
impl sealed::ByteOrder for LittleEndian {}
