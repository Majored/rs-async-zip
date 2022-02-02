// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;

/// An async writer wrapper which tracks the current byte offset.
///
/// This type is only ever used internally to track offsets needed for central directory headers, and to easily
/// calculate compressed & uncompressed file sizes.
pub struct OffsetAsyncWriter<W: AsyncWrite + Unpin> {
    writer: W,
    offset: usize,
}

impl<W: AsyncWrite + Unpin> OffsetAsyncWriter<W> {
    /// Constructs a new offset writer from a generic writer implementing AsyncWrite.
    pub fn from_raw(writer: W) -> Self {
        Self { writer, offset: 0 }
    }

    /// Returns the current writer byte offset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Consumes this writer and returns its inner value.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for OffsetAsyncWriter<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let poll = Pin::new(&mut self.writer).poll_write(cx, buf);

        if let Poll::Ready(Ok(inner)) = poll {
            self.offset += inner;
        }

        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}
