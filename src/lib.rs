//! # Push-based decoding
//!
//! This crate provides abstractions for push-based decoding and pull-based encoding.
//! That means, the caller is responsible for obtaining the bytes to decode and feeding them into
//! decoder or pulling bytes from encoder and feeding them into writr.
//!
//! The main advantage of this approach is that it's IO-agnostic, which implies both
//! **`async`-agnostic** and `no_std`. You can use the same code to deserialize from sync
//! and `async` readers and only need a tiny piece of code to connect the reader to a decoder. This
//! piece of code is provided by this crate for `std`, [`lgio`] (usable with `no_std`), `tokio`, `futures` and `async-std`.
//!
//! # Features
//!
//! * `std` - enables integration with the standard library - it's IO and error traits
//! * `alloc` - enables integration with the standard `alloc` crate
//! * `lgio` - connects decoders to lgio IO.
//! * `tokio` - connects decoders to Tokio IO.
//! * `async-std` - connects decoders to async-std IO.
//! * `futures_0_3` - connects decoders to futures 0.3.x IO

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
#[cfg_attr(test, macro_use)]
extern crate alloc;

#[cfg(any(feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
use core::pin::Pin;

#[cfg(feature = "tokio")]
use actual_tokio as tokio;
#[cfg(feature = "async-std")]
use actual_async_std as async_std;

pub mod decoders;
pub mod encoders;
pub mod error;
pub mod int;
#[cfg(any(feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
pub mod future;
mod macros;

use core::fmt;
use core::ops::ControlFlow;

/// Represents types responsible for decoding bytes pushed into it.
///
/// The types implementing this trait act like state machines (similar to futures) but instead of
/// pulling data from some internal source they receive it in method calls. So they are actually
/// much closer to the traditional state machines than futures.
pub trait Decoder: Sized {
    /// The type of value produced by this decoder.
    type Value;
    /// Decoding error.
    type Error;

    /// Processes nex chunk of bytes and updates the cursor.
    ///
    /// The decoder has to processes the chunk of bytes performing validation and transformation.
    ///
    /// If the bytes are valid the slice is updated to point to unread part. Thus if the slice is
    /// non-epty after this method returns the decoder ended decoding.
    ///
    /// # Errors
    ///
    /// An error is returned in case the bytes are invalid. The validity is defined by the
    /// implementor.
    ///
    /// **No** error may be returned if the number of bytes passed is not sufficient to decode the
    /// value - the remaining bytes will be passed in the following call(s) of this method.
    fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error>;

    /// Called when decoding has ended or there are no more bytes.
    ///
    /// The decoder must validate the bytes passed in so far if it didn't do so yet and return the
    /// decoded value or an error if the bytes were invalid.
    ///
    /// # Errors
    ///
    /// This returns an error if the bytes passed so far are invalid as defined by the decoder.
    /// This commonly happens if the byte stream ended unexpectedly.
    fn end(self) -> Result<Self::Value, Self::Error>;

    /// Processes nex chunk of bytes without updating the cursor.
    ///
    /// This method is usually more convenient for the top-level callers which are receiving bytes
    /// from buffered readers. Instead of modifying the slice this returns the number of bytes
    /// consumed which can be passed to the `consume` method of a buffered reader.
    fn bytes_received(&mut self, mut bytes: &[u8]) -> Result<usize, Self::Error> {
        let prev_len = bytes.len();
        self.decode_chunk(&mut bytes)?;
        Ok(prev_len - bytes.len())
    }

    /// Chains another decoder after this one finishes such that the value of this one is used to
    /// initialize the next one.
    fn then<R: Decoder, F: FnOnce(Self::Value) -> R>(self, fun: F) -> decoders::combinators::Then<Self, R, F> {
        decoders::combinators::Then::new(self, fun)
    }

    /// Chains another decoder after this one finishes such that the value of this one is used to
    /// initialize the next one.
    ///
    /// Unlike `then` this combinator may also return an error and convert the errors into a custom
    /// one.
    fn then_try<E, R: Decoder, F: FnOnce(Self::Value) -> Result<R, E>>(self, fun: F) -> decoders::combinators::ThenTry<E, Self, R, F> where E: From<Self::Error> + From<R::Error> {
        decoders::combinators::ThenTry::new(self, fun)
    }

    /// Chains another decoder after this one to decode two values.
    fn chain<D: Decoder>(self, following: D) -> decoders::combinators::Chain<Self, D> {
        decoders::combinators::Chain::new(self, following)
    }

    /// Resets the decoder returning the decoded value.
    fn take(&mut self) -> Result<Self::Value, Self::Error> where Self: Default {
        core::mem::take(self).end()
    }

    /// Decodes a value from lower-level decoder.
    ///
    /// When multiple decoders are chained one after another in a large state machine this method
    /// can simplify delegation of decoding to the underlying decoder. You can wrap decoding in a
    /// closure passed to [`Self::wrap_sub_decode`] and then just call `sub_decode()?` at the
    /// beginning of each decoding state and continue working with the returned value.
    ///
    /// The method also accepts a function (closure) to convert the errors since using `map_err`
    /// would be annoying because of double wrapping. In case no conversion is desired simply pass
    /// in [`core::convert::identity`].
    ///
    /// Note that this requires the `Default` trait because it resets the decoder every time a
    /// value is decoded. Apart from this resolving borrowing issues it also allows easily decoding
    /// a stream of value in a loop. If you need to work with decoders that require a value (e.g.
    /// [`VecDecoder`](decoders::VecDecoder)) it is recommended to create a specialized decoder that
    /// will decode both (e.g. using [`Then`](decoders::combinators::then)) and call sub_deode on
    /// that.
    ///
    /// You may notice this looks a lot like `await` and in principle it is very similar. The
    /// differences are:
    ///
    /// * `await` also implements the state machine using `Future` trait. This doesn't. The
    ///   `Future::poll` method would have to have another argument for us to be able to use it.
    /// * This returns `ControlFlow` instead of `Poll<Result>` to make it return in case of "not
    ///   ready" as well. The `Try` implementation on `Poll` only returns on `Err`, never on
    ///   `Pending`. This is important for ergonomics.
    /// * While it could be argued the type is morally `Poll<Error>` this one doesn't implement
    ///   `Try` either so it's unsuitable for the purpose.
    fn sub_decode<E, F: FnMut(Self::Error) -> E>(&mut self, bytes: &mut &[u8], mut map_err: F) -> ControlFlow<Result<(), E>, Self::Value> where Self: Default {
        if let Err(error) = self.decode_chunk(bytes) {
            return ControlFlow::Break(Err(map_err(error)));
        }
        if bytes.is_empty() {
            ControlFlow::Break(Ok(()))
        } else {
            match self.take() {
                Ok(value) => ControlFlow::Continue(value),
                Err(error) => ControlFlow::Break(Err(map_err(error))),
            }
        }
    }

    /// Helper for using sub_decode.
    ///
    /// This can be used together with [`sub_decode`](Self::sub_decode) on sub-decoders to make
    /// decoding easier. It helps with type inference and converts `ControlFlow` into `Result`.
    ///
    /// Note that this doesn't allow returning `ControlFlow::Continue` as that wouldn't make sense.
    /// It is recommended to just return `ControlFlow::Break` with the result returned from
    /// `decode_chunk` of the last decoder.
    fn wrap_sub_decode<F: FnOnce() -> ControlFlow<Result<(), Self::Error>, core::convert::Infallible>>(f: F) -> Result<(), Self::Error> {
        match f() {
            ControlFlow::Continue(never) => match never {},
            ControlFlow::Break(result) => result,
        }
    }
}

/// Represents decoders that know a minimum length required for the state to advance.
pub trait KnownMinLenDecoder: Decoder {
    /// Returns the minimum number of bytes known to be consumed by the next `decode_chunk` call.
    ///
    /// If this method returns a number `n` then this method MUST guarantee that the next call to
    /// `decode_chunk` will consume *at least* `n` bytes. Failing to do so may lead to data
    /// corruption. The implementations MUST return zero if *and only if* decoding is at the end.
    ///
    /// The requirement implies that returning a lower number than the known one is valid (if it's
    /// above zero). However it is inefficient to return a lower number than the known one because
    /// it may cause readers to read smaller chunks.
    fn min_required_bytes(&self) -> usize;

    /// Returns true if decoding ended.
    fn is_at_end(&self) -> bool {
        self.min_required_bytes() == 0
    }

    /// Returns a shorter slice if the `buffer` length exceeds `min_required_bytes`.
    ///
    /// This is useful when decoding from an unbuffered reader using an intermediate buffer.
    /// Usually the type `T` should be `u8` or `MaybeUninit<u8>`.
    fn clamp_buffer<'a, T>(&self, buffer: &'a mut [T]) -> &'a mut [T] {
        let min = buffer.len().min(self.min_required_bytes());
        &mut buffer[..min]
    }

    /// Low-level helper for synchronously decoding from unbuffered readers.
    ///
    /// This method takes a reader function responsible for filling the provided buffer with data
    /// and uses it to decode a value. Note that this is slower than buffered reading because of zeroed temporary buffer and intermediate copying.
    /// You should prefer functions like `decode_sync` or, if you really need unbuffered, `decode_sync_unbuffered_with` but if you're implementing
    /// a helper for your own trait/type thi method should be useful.
    ///
    /// The `BUF_LEN` specifies the length of the buffer to use. In general people tend to use
    /// vales of a few kB (e.g. 4096) but if you know the maximum length of decoded chunk for
    /// specific decoder you should pick that one.
    fn sync_decode_with_zeroed_buffer<const BUF_LEN: usize, E, F: FnMut(&mut [u8]) -> Result<usize, E>>(mut self, mut reader: F) -> Result<Self::Value, ReadError<Self::Error, E>> {
        let mut buf = [0u8; BUF_LEN];
        while !self.is_at_end() {
            let buf = self.clamp_buffer(&mut buf);
            let bytes_read = reader(buf).map_err(ReadError::Read)?;
            if bytes_read == 0 {
                break;
            }
            self.bytes_received(&buf[..bytes_read]).map_err(ReadError::Decode)?;
        }
        self.end().map_err(ReadError::Decode)
    }
}

/// Represents types producing bytes of some encoded value.
pub trait Encoder: Sized {
    /// Provides next chunk of encoded bytes.
    ///
    /// The returned bytes represent a part of the value being encoded and should be written to a
    /// writer by the consumer. Empty returned value indicates end - there are no more bytes to be
    /// encoded.
    ///
    /// The returned value MUST be the same for all calls of `encoded_chunk` until
    /// [`next()`](Self::next) is called. IOW it's not allowed to use interior mutability or global
    /// state (randomness) to affect the returned value.
    ///
    /// It's recommended that this method returns bytes within the value if possible or minimal
    /// required buffer otherwise. The *consumers* of the trait are responsible for buffering, so
    /// buffering inside encoder would decrease performance.
    #[must_use = "This method only returns bytes and doesn't modify the target"]
    fn encoded_chunk(&self) -> &[u8];

    /// Advances the state to get the next chunk of encoded bytes.
    ///
    /// Calling this method signals to the encoder that the current chunk was fully processed. The
    /// encoder MUST either:
    ///
    /// * return `true` and provide the next chunk of data in folowing calls to
    ///   [`encoded_chunk`](Self::encoded_chunk).
    /// * return `false` indicating there is no more data.
    ///
    /// The encoder MUST NOT panic or cause similar undesirable behavior if `next` was called again
    /// after it previously returned `false`.  It must simply return `false` again.
    ///
    /// Note that the implementor DOES NOT need to guarantee that `encoded_chunk` is empty after
    /// this method returned `false` and for performance reasons it's not advisable to do in
    /// release builds. The consumers are responsible for handling this situation. The consumers
    /// may use [`track_position`](Self::track_position) to get a more convenient interface handling
    /// these edge-cases and tracking byte position.
    #[must_use = "Relying on encoded_chunk being empty is insufficient"]
    fn next(&mut self) -> bool;

    /// Returns a wrapper that tracks the position of processed bytes.
    ///
    /// The returned wrapper has a bit different interface that is more suitable for writing into
    /// writers. It can handle partial writes and makes handling of end easier. Thus downstream
    /// consumers are encouraged to use it where it makes sense.
    fn track_position(self) -> EncoderPositionTracker<Self> {
        EncoderPositionTracker::new(self)
    }

    fn write_to_slice(mut self, buf: &mut &mut [u8]) -> Result<(), error::BufferOverflow> {
        while !self.encoded_chunk().is_empty() {
            let chunk = self.encoded_chunk();
            if chunk.len() > buf.len() {
                return Err(error::BufferOverflow { bytes_past_end: chunk.len() - buf.len()});
            }
            buf[..chunk.len()].copy_from_slice(chunk);
            *buf = &mut core::mem::take(buf)[chunk.len()..];
            if !self.next() {
                break;
            }
        }
        Ok(())
    }

    /// Writes all encoded bytes to a vec.
    ///
    /// Note that this does **not** call `reserve` since there's no way to know the amount to
    /// reserve. This should be handled by the code producing the decoder instead.
    #[cfg(feature = "alloc")]
    fn write_to_vec(mut self, buf: &mut alloc::vec::Vec<u8>) {
        while !self.encoded_chunk().is_empty() {
            buf.extend_from_slice(self.encoded_chunk());
            if !self.next() {
                break;
            }
        }
    }

    /// Writes all encoded bytes to the `std` writer.
    #[cfg(feature = "std")]
    fn write_all_sync<W: std::io::Write + BufWrite>(mut self, mut writer: W) -> std::io::Result<()> {
        while !self.encoded_chunk().is_empty() {
            writer.write_all(self.encoded_chunk())?;
            if !self.next() {
                break;
            }
        }
        Ok(())
    }

    /// Writes all encoded bytes to the `std` writer.
    #[cfg(feature = "lgio")]
    fn write_all_sync_lgio<W: lgio::BufWrite>(mut self, mut writer: W) -> Result<(), W::WriteError> {
        while !self.encoded_chunk().is_empty() {
            writer.write_all(self.encoded_chunk())?;
            if !self.next() {
                break;
            }
        }
        Ok(())
    }

    /// Writes all encoded bytes to the `tokio` async writer.
    ///
    /// The returned future resolves to `std::io::Result<()>`.
    #[cfg(feature = "tokio")]
    fn write_all_tokio<W: tokio::io::AsyncWrite + BufWrite>(self, writer: W) -> future::TokioEncodeFuture<W, Self> {
        future::TokioEncodeFuture::new(writer, self)
    }

    /// Writes all encoded bytes to the `async-std` async writer.
    ///
    /// The returned future resolves to `std::io::Result<()>`.
    #[cfg(feature = "async-std")]
    fn write_all_async_std<W: async_std::io::Write + BufWrite>(self, writer: W) -> future::AsyncStdEncodeFuture<W, Self> {
        future::AsyncStdEncodeFuture::new(writer, self)
    }

    /// Writes all encoded bytes to the `futures` 0.3 async writer.
    ///
    /// The returned future resolves to `std::io::Result<()>`.
    #[cfg(feature = "futures_0_3")]
    fn write_all_futures_0_3<W: futures_io_0_3::AsyncWrite + BufWrite>(self, writer: W) -> future::Futures0Dot3EncodeFuture<W, Self> {
        future::Futures0Dot3EncodeFuture::new(writer, self)
    }

    /// Chains an encoder constructed by `second_encoder_constructor` after this one.
    ///
    /// This is similar to [`chain`](Self::chain) but only incurs the cost of creating the encoder
    /// if it's actually needed. So if encoding stops before finishing (e.g. due to error) no CPU
    /// time or memory is wasted.
    ///
    /// This will also save memory if the second encoder is larger than `F`.
    ///
    /// Note: `F` needs to be `FnMut` instead of `FnOnce` to correctly handle panics.
    fn then<E: Encoder, F: FnMut() -> E>(self, second_encoder_constructor: F) -> encoders::combinators::Then<Self, E, F> {
        encoders::combinators::Then::new(self, second_encoder_constructor)
    }

    /// Chains another encoder after this one.
    ///
    /// This requires second encoder to be eagerly created which may waste CPU time if encoding
    /// stops early. You should consider using [`then`](Self::then) instead, which may save memory
    /// as well.
    fn chain<T: Encoder>(self, second_encoder: T) -> encoders::combinators::Chain<Self, T> {
        encoders::combinators::Chain::new(self, second_encoder)
    }
}

/// Marker trait for writers that are either buffered or don't incur the cost of context switch.
///
/// The trait should be implemented for types which don't incur a (significant) performance penalty
/// when writing short chunks of data.
#[cfg(any(feature = "std", feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
pub trait BufWrite {}

#[cfg(any(feature = "std", feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
impl<'a, T: BufWrite> BufWrite for &mut T {}

#[cfg(feature = "std")]
impl<T: std::io::Write> BufWrite for std::io::BufWriter<T> {}

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncWrite> BufWrite for tokio::io::BufWriter<T> {}

#[cfg(feature = "async-std")]
impl<T: async_std::io::Write> BufWrite for async_std::io::BufWriter<T> {}

#[cfg(any(feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
pin_project_lite::pin_project! {
    /// Wrapper for external types that are known to be buffered.
    ///
    /// Downstream users may use this to satisfy the constraint of `write_` methods when they
    /// themselves can't implement `BufWrite` for types from external crates due to orphan rules.
    pub struct AssumeBuffered<T> {
        #[pin]
        inner: T
    }
}

/// Wrapper for external types that are known to be buffered.
///
/// Downstream users may use this to satisfy the constraint of `write_` methods when they
/// themselves can't implement `BufWrite` for types from external crates due to orphan rules.
#[cfg(all(feature = "std", not(any(feature = "tokio", feature = "async-std", feature = "futures_0_3"))))]
pub struct AssumeBuffered<T> {
    inner: T,
}

#[cfg(any(feature = "std", feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
impl<T> AssumeBuffered<T> {
    pub fn new(writer: T) -> Self {
        AssumeBuffered {
            inner: writer,
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(any(feature = "std", feature = "tokio", feature = "async-std", feature = "futures_0_3"))]
impl<T> BufWrite for AssumeBuffered<T> {}

#[cfg(feature = "std")]
impl<T: std::io::Write> std::io::Write for AssumeBuffered<T> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(feature = "async-std")]
impl<T: async_std::io::Write> async_std::io::Write for AssumeBuffered<T> {
    fn poll_write(self: Pin<&mut Self>, ctx: &mut core::task::Context, bytes: &[u8]) -> core::task::Poll<std::io::Result<usize>> {
        self.project().inner.poll_write(ctx, bytes)
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut core::task::Context) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_flush(ctx)
    }

    fn poll_close(self: Pin<&mut Self>, ctx: &mut core::task::Context) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_close(ctx)
    }
}

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncWrite> tokio::io::AsyncWrite for AssumeBuffered<T> {
    fn poll_write(self: Pin<&mut Self>, ctx: &mut core::task::Context, bytes: &[u8]) -> core::task::Poll<std::io::Result<usize>> {
        self.project().inner.poll_write(ctx, bytes)
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut core::task::Context) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_flush(ctx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, ctx: &mut core::task::Context) -> core::task::Poll<std::io::Result<()>> {
        self.project().inner.poll_shutdown(ctx)
    }
}

/// An `Encoder` wrapper that handles partial writes.
///
/// This wrapper internally tracks the position of encoded bytes which makes handling of partial
/// writes very easy. It also simplifies handling of the end.
#[derive(Debug, Clone)]
pub struct EncoderPositionTracker<Enc> {
    encoder: Enc,
    pos: usize,
}

impl<Enc: Encoder> EncoderPositionTracker<Enc> {
    fn new(encoder: Enc) -> Self {
        EncoderPositionTracker {
            encoder,
            pos: 0,
        }
    }

    /// Returns an unprocessed chunk of encoded data.
    ///
    /// The returned bytes should be processed - e.g. by writing them into a writer. Empty returned
    /// value indicates there are no more bytes.
    #[must_use = "This method only returns bytes and doesn't modify the target"]
    pub fn encoded_chunk(&self) -> &[u8] {
        &self.encoder.encoded_chunk()[self.pos..]
    }

    /// Marks the `amount` of bytes as processed.
    ///
    /// The consumer should call this method after (partially) processing the chunk. Usually this
    /// is called after successful [`write`](std::io::Write::write) or analogous function. The
    /// buffer returned by [`encoded_chunk`](Self::encoded_chunk) will by advanced by `amount`
    /// bytes and, if it reached the end, the underlying encoder will be advanced to give the next
    /// chunk.
    ///
    /// Calling this method with `amount` larger than `encoded_chunk().len()` will corrupt the
    /// encoder and lead to a panic later. In debug builds this will panic instantly.
    #[track_caller]
    pub fn consume(&mut self, amount: usize) {
        self.pos += amount;
        if self.pos >= self.encoder.encoded_chunk().len() {
            debug_assert_eq!(self.pos, self.encoder.encoded_chunk().len());
            // Resetting only position when there are more chunks ensures `encoded_bytes` will be
            // empty when this reaches the end.
            if self.encoder.next() {
                self.pos = 0;
            }
        }
    }

    /// Issues single write to te writer and advances the position accordingly.
    ///
    /// This can be used as a building block for various abstractions or protocols.
    /// Returns the number of bytes written. Zero indicates the end of encoding.
    #[cfg(feature = "std")]
    pub fn write_once<W: std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<usize> {
        if self.encoded_chunk().is_empty() {
            return Ok(0);
        }
        let amount = writer.write(self.encoded_chunk())?;
        self.consume(amount);
        Ok(amount)
    }

    /// Writes all bytes to the writer until the end or an error.
    ///
    /// This is similar to `Encoder::write_all` with one significant difference: it leaves the
    /// state around so the operation can be restarted. This can be used to handle
    /// [`ErrorKind::Interrupted`](std::io::ErrorKind::Interrupted) errors which are generally
    /// recoverable but users may still wish to act on them (e.g. check a global flag set by a
    /// signal).
    #[cfg(feature = "std")]
    pub fn write_all<W: std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        while self.write_once(writer)? != 0 { }
        Ok(())
    }
}

/// Synchronously decodes a value from the given reader using a custom decoder.
#[cfg(feature = "std")]
pub fn decode_sync_with<D: Decoder, R: std::io::BufRead + ?Sized>(reader: &mut R, mut decoder: D) -> Result<D::Value, ReadError<D::Error>> {
    loop {
        let buf = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ReadError::Read(error)),
        };
        if buf.is_empty() {
            break decoder.end().map_err(ReadError::Decode);
        }
        let num = decoder.bytes_received(buf).map_err(ReadError::Decode)?;
        let buf_len = buf.len();
        reader.consume(num);
        if num < buf_len {
            break decoder.end().map_err(ReadError::Decode);
        }
    }
}

/// Synchronously decodes a value from the given reader using a slower algorithm.
///
/// Because this function doesn't use `BufRead` it has to make an intermediate copy that might or
/// might not be optimized-out. It also has to make a temporary zeroed buffer because of `std`
/// limitations and zeroing might not be optimized out. Also if the reader is a truly unbuffered OS
/// resource this will be painfully slow for many common decoders as it'll make many syscalls.
///
/// The function is only provided for compatibility with poorly-designed APIs that cannot use
/// `BufRead` for some reason.
#[cfg(feature = "std")]
pub fn decode_sync_unbuffered_with<const BUF_LEN: usize, D: KnownMinLenDecoder, R: std::io::Read + ?Sized>(reader: &mut R, decoder: D) -> Result<D::Value, ReadError<D::Error>> {
    decoder.sync_decode_with_zeroed_buffer::<BUF_LEN, _, _>(move |buf| reader.read(buf))
}

/// Synchronously decodes a value from the given reader.
#[cfg(feature = "std")]
pub fn decode_sync<D: Decoder + Default>(reader: &mut (impl std::io::BufRead + ?Sized)) -> Result<D::Value, ReadError<D::Error>> {
    decode_sync_with(reader, D::default())
}

/// Synchronously decodes a value from the given reader using a custom decoder.
#[cfg(feature = "lgio")]
pub fn decode_sync_lgio_with<D: Decoder, R: lgio::BufRead + ?Sized>(reader: &mut R, mut decoder: D) -> Result<D::Value, ReadError<D::Error, R::ReadError>> {
    loop {
        let buf = reader.fill_buf().map_err(ReadError::Read)?;
        if buf.is_empty() {
            break decoder.end().map_err(ReadError::Decode);
        }
        let num = decoder.bytes_received(buf).map_err(ReadError::Decode)?;
        let buf_len = buf.len();
        reader.consume(num);
        if num < buf_len {
            break decoder.end().map_err(ReadError::Decode);
        }
    }
}

/// Synchronously decodes a value from the given reader.
#[cfg(feature = "lgio")]
pub fn decode_sync_lgio<D: Decoder + Default, R: lgio::BufRead + ?Sized>(reader: &mut R) -> Result<D::Value, ReadError<D::Error, R::ReadError>> {
    decode_sync_lgio_with(reader, D::default())
}

/// Asynchronously decodes a value from the given reader using a custom decoder.
#[cfg(feature = "futures_0_3")]
pub async fn decode_futures_0_3_with<D: Decoder, R: futures_io_0_3::AsyncBufRead>(reader: R, decoder: D) -> Result<D::Value, ReadError<D::Error>> {
    use futures_io_0_3::AsyncBufRead;

    future::DecodeFuture {
        reader,
        poll_fn: <R as AsyncBufRead>::poll_fill_buf,
        consume_fn: <R as AsyncBufRead>::consume,
        decoder: Some(decoder),
    }
    .await
}

/// Asynchronously decodes a value from the given reader.
#[cfg(feature = "futures_0_3")]
pub async fn decode_futures_0_3<D: Decoder + Default>(reader: impl futures_io_0_3::AsyncBufRead) -> Result<D::Value, ReadError<D::Error>> {
    decode_futures_0_3_with(reader, D::default()).await
}

/// Asynchronously decodes a value from the given reader using a custom decoder.
#[cfg(feature = "tokio")]
pub async fn decode_tokio_with<D: Decoder, R: tokio::io::AsyncBufRead>(reader: R, decoder: D) -> Result<D::Value, ReadError<D::Error>> {
    use tokio::io::AsyncBufRead;

    future::DecodeFuture {
        reader,
        poll_fn: <R as AsyncBufRead>::poll_fill_buf,
        consume_fn: <R as AsyncBufRead>::consume,
        decoder: Some(decoder),
    }
    .await
}

/// Asynchronously decodes a value from the given reader.
#[cfg(feature = "tokio")]
pub async fn decode_tokio<D: Decoder + Default>(reader: impl tokio::io::AsyncBufRead) -> Result<D::Value, ReadError<D::Error>> {
    decode_tokio_with(reader, D::default()).await
}

/// Asynchronously decodes a value from the given reader using a custom decoder.
#[cfg(feature = "async-std")]
pub async fn decode_async_std_with<D: Decoder, R: async_std::io::BufRead>(reader: R, decoder: D) -> Result<D::Value, ReadError<D::Error>> {
    use async_std::io::BufRead as AsyncBufRead;

    future::DecodeFuture {
        reader,
        poll_fn: <R as AsyncBufRead>::poll_fill_buf,
        consume_fn: <R as AsyncBufRead>::consume,
        decoder: Some(decoder),
    }
    .await
}

/// Asynchronously decodes a value from the given reader.
#[cfg(feature = "async-std")]
pub async fn decode_async_std<D: Decoder + Default>(reader: impl async_std::io::BufRead) -> Result<D::Value, ReadError<D::Error>> {
    decode_async_std_with(reader, D::default()).await
}

/// Returned when either reading or decoding fails.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
// A trick to hide (wrong) cfg doc
#[cfg_attr(docsrs, doc(cfg(all())))]
#[cfg(not(feature = "std"))]
pub enum ReadError<Decode, Read> {
    /// Reading from a reader failed.
    Read(Read),
    /// Decoding the value failed.
    Decode(Decode),
}

/// Returned when either reading or decoding fails.
///
/// Note that the `Read` type param only defaults to [`std::io::Error`] with `std` feature enabled.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
// A trick to hide (wrong) cfg doc
#[cfg_attr(docsrs, doc(cfg(all())))]
#[cfg(feature = "std")]
pub enum ReadError<Decode, Read = std::io::Error> {
    /// Reading from a reader failed.
    Read(Read),
    /// Decoding the value failed.
    Decode(Decode),
}

impl<D, R> ReadError<D, R> {
    /// Converts the inner errors to another type `E`.
    pub fn convert_either<E>(self) -> E where D: Into<E>, R: Into<E> {
        match self {
            ReadError::Read(error) => error.into(),
            ReadError::Decode(error) => error.into(),
        }
    }

    /// Converts the read error using a closure.
    ///
    /// This is analogous to [`Result::map`]/[`Result::map_err`] and leaves `Decode` intact.
    pub fn map_read<E, F>(self, map: F) -> ReadError<D, E> where F: FnOnce(R) -> E {
        match self {
            ReadError::Read(error) => ReadError::Read(map(error)),
            ReadError::Decode(error) => ReadError::Decode(error),
        }
    }

    /// Converts the decode error using a closure.
    ///
    /// This is analogous to [`Result::map`]/[`Result::map_err`] and leaves `Read` intact.
    pub fn map_decode<E, F>(self, map: F) -> ReadError<E, R> where F: FnOnce(D) -> E {
        match self {
            ReadError::Read(error) => ReadError::Read(error),
            ReadError::Decode(error) => ReadError::Decode(map(error)),
        }
    }
}

impl<E> ReadError<E, core::convert::Infallible> {
    /// Statically proves that reading is infallible and converts to decode error.
    pub fn into_decode(self) -> E {
        match self {
            ReadError::Read(never) => match never {},
            ReadError::Decode(error) => error,
        }
    }
}

impl<E> ReadError<core::convert::Infallible, E> {
    /// Statically proves that decoding is infallible and converts to read error.
    pub fn into_read(self) -> E {
        match self {
            ReadError::Read(error) => error,
            ReadError::Decode(never) => match never {},
        }
    }
}

impl From<ReadError<core::convert::Infallible, core::convert::Infallible>> for core::convert::Infallible {
    fn from(error: ReadError<core::convert::Infallible, core::convert::Infallible>) -> Self {
        match error {
            ReadError::Read(error) => error,
            ReadError::Decode(error) => error,
        }
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + Send + Sync + 'static> From<ReadError<E, std::io::Error>> for std::io::Error {
    fn from(error: ReadError<E, std::io::Error>) -> Self {
        use std::io::ErrorKind;

        match error {
            ReadError::Read(error) => error,
            ReadError::Decode(error) => std::io::Error::new(ErrorKind::InvalidData, error),
        }
    }
}

impl<D, R> fmt::Display for ReadError<D, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReadError::Read(_) => write!(f, "reading failed"),
            ReadError::Decode(_) => write!(f, "decoding failed"),
        }
    }
}

#[cfg(feature = "std")]
impl<D: std::error::Error + 'static, R: std::error::Error + 'static> std::error::Error for ReadError<D, R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Read(error) => Some(error),
            ReadError::Decode(error) => Some(error),
        }
    }
}
