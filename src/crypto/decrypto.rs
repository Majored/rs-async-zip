// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::crypto::crypto::{ENCRYPTION_HEADER_SIZE, ZipCrypto};
use crate::error::ZipError;

use std::io::ErrorKind;
use std::pin::Pin;
use std::task::ready;
use std::task::{Context, Poll};

use futures_lite::io::{AsyncBufRead, AsyncRead};
use pin_project::pin_project;

/// How many ciphertext bytes are pulled from the inner reader per round of decryption.
const READ_CHUNK: usize = 2048;

/// Per-reader decryption state, only present when a password was provided.
struct CryptState {
    cipher: ZipCrypto,
    /// The expected value of the final decrypted encryption header byte, used to verify the
    /// password. This is the high byte of the CRC32 for files without a data descriptor, and the
    /// high byte of the last modification time otherwise (as per PKWARE's APPNOTE).
    check_byte: u8,
    /// Whether the encryption header has been consumed and the password verified. The header is
    /// processed exactly once, as running it through the cipher a second time would desynchronise
    /// the keystream.
    verified: bool,
    /// The encrypted encryption header as read from the inner reader.
    header: [u8; ENCRYPTION_HEADER_SIZE],
    header_len: usize,
    /// Decrypted bytes which have not yet been handed to the caller.
    buf: Vec<u8>,
    pos: usize,
}

impl CryptState {
    /// Pulls more ciphertext from the inner reader, feeding the encryption header through the
    /// cipher first, and fills the decrypted buffer. Returns once the buffer holds at least one
    /// byte, or EOF is reached.
    fn poll_fill<R: AsyncRead>(&mut self, mut inner: Pin<&mut R>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.pos < self.buf.len() {
            return Poll::Ready(Ok(()));
        }

        self.buf.clear();
        self.pos = 0;

        if !self.verified {
            while self.header_len < ENCRYPTION_HEADER_SIZE {
                let read = ready!(AsyncRead::poll_read(inner.as_mut(), cx, &mut self.header[self.header_len..]))?;
                if read == 0 {
                    return Poll::Ready(Err(std::io::Error::new(
                        ErrorKind::UnexpectedEof,
                        "truncated ZipCrypto encryption header",
                    )));
                }
                self.header_len += read;
            }

            let mut last = 0u8;
            for &byte in self.header.iter() {
                last = self.cipher.decrypt_byte(byte);
            }

            if last != self.check_byte {
                return Poll::Ready(Err(std::io::Error::new(ErrorKind::InvalidData, ZipError::InvalidPassword)));
            }

            self.verified = true;
        }

        let mut chunk = [0u8; READ_CHUNK];
        let read = ready!(AsyncRead::poll_read(inner.as_mut(), cx, &mut chunk))?;
        if read == 0 {
            return Poll::Ready(Ok(()));
        }

        self.buf.extend_from_slice(&chunk[..read]);
        for byte in self.buf.iter_mut() {
            *byte = self.cipher.decrypt_byte(*byte);
        }

        Poll::Ready(Ok(()))
    }
}

/// A wrapping reader which decrypts ZipCrypto-encrypted data before passing it to the caller.
///
/// The 12-byte encryption header which precedes the ciphertext is consumed transparently and its
/// check byte verified against the expected value, failing early with
/// [`ZipError::InvalidPassword`] if the password is incorrect.
#[pin_project(project = DecryptingReaderProj)]
pub(crate) struct DecryptingReader<R> {
    #[pin]
    inner: R,
    state: Option<CryptState>,
}

impl<R> DecryptingReader<R> {
    /// Constructs a new wrapping reader which decrypts data with the given password.
    ///
    /// `check_byte` is the expected final decrypted byte of the encryption header, used to verify
    /// the password. See [`CryptState`].
    pub(crate) fn new(inner: R, password: &[u8], check_byte: u8) -> Self {
        Self {
            inner,
            state: Some(CryptState {
                cipher: ZipCrypto::new(password),
                check_byte,
                verified: false,
                header: [0; ENCRYPTION_HEADER_SIZE],
                header_len: 0,
                buf: Vec::new(),
                pos: 0,
            }),
        }
    }

    /// Constructs a new wrapping reader which passes data through unmodified.
    pub(crate) fn passthrough(inner: R) -> Self {
        Self { inner, state: None }
    }

    /// Consumes this wrapper and returns the inner reader.
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for DecryptingReader<R> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let mut this = self.project();

        let Some(state) = this.state.as_mut() else {
            return this.inner.poll_read(cx, buf);
        };

        ready!(CryptState::poll_fill(state, this.inner.as_mut(), cx))?;

        let remaining = &state.buf[state.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        state.pos += n;

        Poll::Ready(Ok(n))
    }
}

impl<R: AsyncBufRead + Unpin> AsyncBufRead for DecryptingReader<R> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let mut this = self.project();

        let Some(state) = this.state.as_mut() else {
            return this.inner.poll_fill_buf(cx);
        };

        ready!(state.poll_fill(this.inner.as_mut(), cx))?;
        Poll::Ready(Ok(&state.buf[state.pos..]))
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.project();

        let Some(state) = this.state.as_mut() else {
            return this.inner.consume(amt);
        };

        state.pos = (state.pos + amt).min(state.buf.len());
    }
}
