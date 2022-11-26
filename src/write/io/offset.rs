// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-io-utilities/blob/main/LICENSE)

use std::io::{Error, IoSlice};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;
use tokio::io::AsyncWrite;

/// A wrapper around an [`AsyncWrite`] implementation which tracks the current byte offset.
#[pin_project(project = OffsetWriterProj)]
pub struct AsyncOffsetWriter<W>
where
    W: AsyncWrite + Unpin,
{
    #[pin]
    inner: W,
    offset: usize,
}

impl<W> AsyncOffsetWriter<W>
where
    W: AsyncWrite + Unpin,
{
    /// Constructs a new wrapper from an inner [`AsyncWrite`] writer.
    pub fn new(inner: W) -> Self {
        Self { inner, offset: 0 }
    }

    /// Returns the current byte offset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Consumes this wrapper and returns the inner [`AsyncWrite`] writer.
    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn inner_mut(&mut self) -> &mut W {
	&mut self.inner
    }
}

impl<W> AsyncWrite for AsyncOffsetWriter<W>
where
    W: AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let this = self.project();
        let poll = this.inner.poll_write(cx, buf);

        if let Poll::Ready(Ok(inner)) = &poll {
            *this.offset += inner;
        }

        poll
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        self.project().inner.poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<Result<usize, Error>> {
        self.project().inner.poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}
