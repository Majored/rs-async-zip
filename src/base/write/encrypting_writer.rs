// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::crypto::crypto::ZipCrypto;

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_lite::io::AsyncWrite;

/// A wrapping writer which encrypts data with a [`ZipCrypto`] instance before passing it to the
/// inner writer.
///
/// # Note
/// - The cipher's key state must advance by exactly the number of ciphertext bytes that were
///   successfully accepted by the inner writer. Partial writes and `Pending` results therefore
///   roll the key state back to the point before the current buffer was encrypted, so that a
///   retry re-encrypts the same plaintext from the same key state.
/// - Callers are expected to pass a cipher whose key state is already positioned after the
///   12-byte encryption header (ie. the same instance used to generate that header), keeping
///   the keystream continuous between the header and the data body.
pub(crate) struct EncryptingWriter<W: AsyncWrite + Unpin> {
    inner: W,
    crypto: Option<ZipCrypto>,
    buf: Vec<u8>,
}

impl<W: AsyncWrite + Unpin> EncryptingWriter<W> {
    /// Constructs a new wrapping writer from an inner writer and an optional cipher.
    ///
    /// A `None` cipher passes data through unencrypted.
    pub(crate) fn new(inner: W, crypto: Option<ZipCrypto>) -> Self {
        Self { inner, crypto, buf: Vec::new() }
    }

    /// Consumes this wrapper and returns the inner writer.
    pub(crate) fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for EncryptingWriter<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        let this = &mut *self;

        let crypto = match this.crypto.as_mut() {
            Some(crypto) => crypto,
            None => return Pin::new(&mut this.inner).poll_write(cx, buf),
        };

        // Snapshot the key state so that it can be rolled back if the inner writer accepts
        // fewer bytes than prepared this round.
        let snapshot = crypto.save_keys();

        this.buf.clear();
        this.buf.extend_from_slice(buf);
        crypto.encrypt_data(&mut this.buf);

        let poll = Pin::new(&mut this.inner).poll_write(cx, &this.buf);
        let encrypted_len = this.buf.len();

        match poll {
            // Full write (including a zero-length one): the key state already covers exactly
            // the accepted ciphertext.
            Poll::Ready(Ok(n)) if n >= encrypted_len => poll,
            // Partial write: rewind to the snapshot and advance only over the plaintext that
            // corresponds to the accepted ciphertext.
            Poll::Ready(Ok(n)) => {
                crypto.restore_keys(snapshot);
                crypto.advance_keys(&buf[..n]);
                poll
            }
            // Nothing was accepted this round; rewind so the next attempt re-encrypts the
            // same plaintext from the same key state.
            Poll::Pending => {
                crypto.restore_keys(snapshot);
                poll
            }
            Poll::Ready(Err(_)) => poll,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::io::AsyncWriteExt;

    /// A writer which returns `Pending` a set number of times before each success and then
    /// accepts at most `accept` bytes per poll, to exercise partial-write handling.
    struct ChunkedWriter {
        sink: Vec<u8>,
        accept: usize,
        delay: u32,
    }

    impl AsyncWrite for ChunkedWriter {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
            if self.delay > 0 {
                self.delay -= 1;
                // Reschedule immediately so `block_on` retries instead of waiting forever
                // for a wakeup that would never come from this no-op sink.
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let n = buf.len().min(self.accept);
            self.sink.extend_from_slice(&buf[..n]);
            Poll::Ready(Ok(n))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
            Poll::Ready(Ok(()))
        }
    }

    /// The cipher key state must stay in sync with the accepted ciphertext even when the inner
    /// writer returns `Pending` and accepts partial chunks.
    #[test]
    fn encrypting_writer_survives_partial_and_pending_writes() {
        let password = b"unit-test-password";
        let plaintext: Vec<u8> = (0..1000u32).map(|i| (i * 31 % 251) as u8).collect();

        let mut writer = EncryptingWriter::new(ChunkedWriter { sink: Vec::new(), accept: 7, delay: 2 }, Some(ZipCrypto::new(password)));

        futures_lite::future::block_on(async {
            // Multiple writes of uneven sizes, plus the chunked inner writer forcing many
            // partial writes and `Pending` polls in between.
            for chunk in plaintext.chunks(253) {
                writer.write_all(chunk).await.unwrap();
            }
        });

        let sink = writer.into_inner().sink;
        assert_eq!(sink.len(), plaintext.len());

        // Decrypt with a fresh cipher and compare against the original plaintext.
        let mut decryptor = ZipCrypto::new(password);
        let decrypted: Vec<u8> = sink.iter().map(|&b| decryptor.decrypt_byte(b)).collect();
        assert_eq!(decrypted, plaintext);
    }

    /// Without a cipher the wrapper must behave as a plain pass-through.
    #[test]
    fn encrypting_writer_passthrough_without_crypto() {
        let data = b"plain passthrough";
        let mut writer = EncryptingWriter::new(Vec::new(), None);

        futures_lite::future::block_on(async {
            writer.write_all(data).await.unwrap();
        });

        assert_eq!(writer.into_inner(), data.to_vec());
    }
}
