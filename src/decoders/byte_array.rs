use core::mem::MaybeUninit;
use crate::{Decoder, KnownMinLenDecoder};
use crate::error::UnexpectedEnd;

#[derive(Debug)]
pub struct ByteArrayDecoder<const N: usize> {
    buf: [MaybeUninit<u8>; N],
    len: usize,
}

impl<const N: usize> ByteArrayDecoder<N> {
    pub fn new() -> Self {
        ByteArrayDecoder {
            buf: [MaybeUninit::uninit(); N],
            len: 0,
        }
    }
}

impl<const N: usize> Default for ByteArrayDecoder<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Decoder for ByteArrayDecoder<N> {
    type Value = [u8; N];
    type Error = UnexpectedEnd;

    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
        fn cast_slice(slice: &[u8]) -> &[MaybeUninit<u8>] {
            // SAFETY: the pointer is alive, the lifetimes match, length matches, the layout of
            // `MaybeUninit<T>` is same as `T`, the returned reference is immutable so the caller
            // cannot abuse it to write `MaybeUninit::uninit` into the slice.
            unsafe { core::slice::from_raw_parts(slice.as_ptr().cast(), slice.len()) }
        }
        let to_copy = bytes.len().min(N - self.len);
        self.buf[self.len..(self.len + to_copy)].copy_from_slice(cast_slice(&bytes[..to_copy]));
        self.len += to_copy;
        *bytes = &bytes[to_copy..];
        Ok(())
    }

    fn end(self) -> Result<Self::Value, Self::Error> {
        /// `std` version is not stable, this is pretty much a copy.
        ///
        /// SAFETY: requires the `array` to be entirely overwritten (initialized).
        unsafe fn array_assume_init<const LEN: usize>(array: [MaybeUninit<u8>; LEN]) -> [u8; LEN] {
            // SAFETY: the layouts of elements are the same, the lengths are the same, caller may
            // only call it on initialized array.
            core::mem::transmute_copy(&array)
        }
        if self.len < N {
            Err(UnexpectedEnd { missing: N - self.len })
        } else {
            Ok(unsafe { array_assume_init(self.buf) })
        }
    }
}

impl<const N: usize> KnownMinLenDecoder for ByteArrayDecoder<N> {
    fn min_required_bytes(&self) -> usize {
        N - self.len
    }
}

#[cfg(test)]
mod tests {
    use super::ByteArrayDecoder;
    use crate::Decoder;

    #[test]
    fn empty() {
        let mut decoder = ByteArrayDecoder::<0>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 0);
        decoder.end().unwrap();
    }

    #[test]
    fn empty_immediate_end() {
        let decoder = ByteArrayDecoder::<0>::new();
        decoder.end().unwrap();
    }

    #[test]
    fn one() {
        let mut decoder = ByteArrayDecoder::<1>::new();
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn one_immediate_end() {
        let mut decoder = ByteArrayDecoder::<1>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42]);
    }

    #[test]
    fn two_single() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42, 21]).unwrap(), 2);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }

    #[test]
    fn two_split_extra() {
        let mut decoder = ByteArrayDecoder::<2>::new();
        assert_eq!(decoder.bytes_received(&[42]).unwrap(), 1);
        assert_eq!(decoder.bytes_received(&[21, 47]).unwrap(), 1);
        assert_eq!(decoder.end().unwrap(), [42, 21]);
    }
}
