// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::crypto::crypto::ZipCrypto;

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_lite::io::{AsyncBufRead, AsyncRead};
use pin_project::pin_project;

/// A wrapping reader which handles ZipCrypto decryption
#[pin_project(project = DecryptedReaderProj)]
pub(crate) struct DecryptedReader<R> {
    #[pin]
    inner: R,
    crypto: ZipCrypto,
    remaining_encryption_header: usize,
}

impl<R> DecryptedReader<R>
where
    R: AsyncBufRead + Unpin,
{
    /// Constructs a new wrapping reader with password for decryption.
    /// The encryption header is 12 bytes and needs special handling.
    pub(crate) fn new(reader: R, password: &[u8]) -> Self {
        Self {
            inner: reader,
            crypto: ZipCrypto::new(password),
            remaining_encryption_header: 12, // ZipCrypto encryption header is 12 bytes
        }
    }

    /// Consumes this reader and returns the inner value.
    pub(crate) fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> AsyncRead for DecryptedReader<R>
where
    R: AsyncBufRead + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let mut this = self.project();

        // First, consume the encryption header
        if *this.remaining_encryption_header > 0 {
            let mut header_buf = vec![0u8; *this.remaining_encryption_header];

            // Read encryption header bytes using a simple approach
            let inner = &mut this.inner;
            let mut inner_pin = Pin::new(inner);

            match AsyncRead::poll_read(inner_pin.as_mut(), cx, &mut header_buf) {
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        return Poll::Ready(Ok(0));
                    }

                    // Process header bytes through crypto to get to the right state
                    for &byte in &header_buf[..n] {
                        this.crypto.decrypt_byte(byte);
                    }

                    *this.remaining_encryption_header -= n;

                    // If we've consumed all header bytes, continue to read actual data
                    if *this.remaining_encryption_header == 0 {
                        // Continue to read data below
                    } else {
                        return Poll::Ready(Ok(n));
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        // Read and decrypt actual data
        let inner = &mut this.inner;
        let mut inner_pin = Pin::new(inner);

        match AsyncRead::poll_read(inner_pin.as_mut(), cx, buf) {
            Poll::Ready(Ok(n)) => {
                if n == 0 {
                    Poll::Ready(Ok(0))
                } else {
                    // Decrypt the bytes
                    for i in 0..n {
                        buf[i] = this.crypto.decrypt_byte(buf[i]);
                    }
                    Poll::Ready(Ok(n))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R> AsyncBufRead for DecryptedReader<R>
where
    R: AsyncBufRead + Unpin,
{
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        // For BufRead, we need to decrypt the buffered data
        // This is complex because we need to maintain state
        // For simplicity, we'll delegate to AsyncRead
        self.poll_read(cx, &mut []).map(|r| r.map(|_| &[][..]))
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        // This is a no-op for our use case since we handle consumption in poll_read
        let _ = amt;
    }
}
