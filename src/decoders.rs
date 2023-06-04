pub mod combinators;

#[cfg(rust_v_1_51)]
mod byte_array;
mod u8_decoder;
#[cfg(rust_v_1_51)]
mod int;

#[cfg(feature = "alloc")]
mod byte_vec;

#[cfg(feature = "alloc")]
mod utf8_string;

#[cfg(rust_v_1_51)]
pub use byte_array::ByteArrayDecoder;
pub use u8_decoder::U8Decoder;
#[cfg(rust_v_1_51)]
pub use int::*;

#[cfg(feature = "alloc")]
pub use byte_vec::ByteVecDecoder;

#[cfg(feature = "alloc")]
pub use utf8_string::Utf8StringDecoder;

#[cfg(feature = "alloc")]
pub use utf8_string::Error as Utf8StringError;
