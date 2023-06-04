use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;
use std::io;
use super::{Decoder, Encoder, EncoderPositionTracker, ReadError};

macro_rules! impl_encoder {
    ($future:ident, $trait:path, $poll_write:ident) => {
        pin_project! {
            pub struct $future<W: $trait, E: Encoder> {
                #[pin]
                inner: EncodeFuture<W, $poll_write, E>
            }
        }

        impl<W: $trait, E: Encoder> $future<W, E> {
            pub(crate) fn new(writer: W, encoder: E) -> Self {
                $future {
                    inner: EncodeFuture {
                        writer,
                        encoder: encoder.track_position(),
                        poll_write_fn: $poll_write,
                    },
                }
            }
        }

        impl<W: $trait, E: Encoder> Future for $future<W, E> {
            type Output = io::Result<()>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
                self.project().inner.poll(ctx)
            }
        }

        struct $poll_write;

        impl<Writer: $trait> PollWrite<Writer> for $poll_write {
            fn poll_write(writer: Pin<&mut Writer>, ctx: &mut Context, bytes: &[u8]) -> Poll<io::Result<usize>> {
                writer.poll_write(ctx, bytes)
            }
        }
    }
}

#[cfg(feature = "tokio")]
impl_encoder!(TokioEncodeFuture, actual_tokio::io::AsyncWrite, TokioPollWrite);

#[cfg(feature = "async-std")]
impl_encoder!(AsyncStdEncodeFuture, actual_async_std::io::Write, AsyncStdPollWrite);

#[cfg(feature = "futures_0_3")]
impl_encoder!(Futures0Dot3EncodeFuture, futures_io_0_3::AsyncWrite, Futures0Dot3PollWrite);

pin_project! {
    pub(crate) struct DecodeFuture<T, PollFn, ConsumeFn, D: Decoder> {
        #[pin]
        pub(crate) reader: T,
        pub(crate) poll_fn: PollFn,
        pub(crate) consume_fn: ConsumeFn,
        pub(crate) decoder: Option<D>,
    }
}

impl<T, PollFn, ConsumeFn, D> Future for DecodeFuture<T, PollFn, ConsumeFn, D> where
    PollFn: for<'a> FnMut(Pin<&'a mut T>, &mut Context) -> Poll<io::Result<&'a [u8]>>,
    ConsumeFn: FnMut(Pin<&mut T>, usize),
    D: Decoder,
{
    type Output = Result<D::Value, ReadError<D::Error>>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let buf = match (this.poll_fn)(this.reader.as_mut(), ctx) {
                Poll::Ready(Ok(buf)) => buf,
                Poll::Ready(Err(error)) => return Poll::Ready(Err(ReadError::Read(error))),
                Poll::Pending => return Poll::Pending,
            };
            if buf.is_empty() {
                let result = this.decoder
                    .take()
                    .expect("resolved future polled again")
                    .end()
                    .map_err(ReadError::Decode);

                return Poll::Ready(result);
            }
            let buf_len = buf.len();
            let result = this.decoder
                .as_mut()
                .expect("resolved future polled again")
                .bytes_received(buf);
            let num = match result {
                Ok(num) => num,
                Err(error) => return Poll::Ready(Err(ReadError::Decode(error))),
            };
            (this.consume_fn)(this.reader.as_mut(), num);
            if num < buf_len {
                return Poll::Ready(this.decoder.take().unwrap().end().map_err(ReadError::Decode));
            }
        }
    }
}

pin_project! {
    pub(crate) struct EncodeFuture<T, PollWriteFn, E: Encoder> {
        #[pin]
        pub(crate) writer: T,
        pub(crate) poll_write_fn: PollWriteFn,
        pub(crate) encoder: EncoderPositionTracker<E>,
    }
}

impl<T, PollWriteFn, E> Future for EncodeFuture<T, PollWriteFn, E> where
    PollWriteFn: PollWrite<T>,
    E: Encoder,
{
    type Output = std::io::Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut this = self.project();

        while !this.encoder.encoded_chunk().is_empty() {
            match PollWriteFn::poll_write(this.writer.as_mut(), ctx, this.encoder.encoded_chunk()) {
                Poll::Ready(Ok(amount)) => this.encoder.consume(amount),
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }
}

// Can't use FnMut because fns are unnameable.
trait PollWrite<Writer> {
    fn poll_write(writer: Pin<&mut Writer>, ctx: &mut Context, bytes: &[u8]) -> Poll<io::Result<usize>>;
}
