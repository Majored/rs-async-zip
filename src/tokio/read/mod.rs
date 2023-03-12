// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod fs;
pub mod seek;
pub mod stream;

use std::{pin::Pin, task::{Context, Poll}};

use crate::{base::read::io::entry::ZipEntryReader as BaseZipEntryReader, ZipEntry, error::Result};
use pin_project::pin_project;
use tokio::io::{AsyncRead, ReadBuf};
use tokio_util::compat::Compat;
use futures_util::AsyncRead as FuturesAsyncRead;

/// A ZIP entry reader which may implement decompression.
#[pin_project]
pub struct ZipEntryReader<'a, R>(#[pin] BaseZipEntryReader<'a, Compat<R>>);

impl<'a, R> AsyncRead for ZipEntryReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        // TODO: This is pulled from: https://github.com/tokio-rs/tokio/blob/e34978233bfff02cce57a17a9a2c6583943380ea/tokio-util/src/compat.rs#L112
        //       Can we directly use it rather than implement it ourselves?

        let result = self.project().0.poll_read(c, b.initialize_unfilled());

        if let Poll::Ready(Ok(advance)) = &result {
            b.advance(*advance);
        }

        result.map_ok(|_| ())
    }
}

impl<'a, R> ZipEntryReader<'a, R>
where
    R: AsyncRead + Unpin,
{
    /// Computes and returns the CRC32 hash of bytes read by this reader so far.
    ///
    /// This hash should only be computed once EOF has been reached.
    fn compute_hash(&mut self) -> u32 {
        self.0.compute_hash()
    }

    /// Reads all bytes until EOF has been reached, appending them to buf, and verifies the CRC32 values.
    ///
    /// This is a helper function synonymous to [`AsyncReadExt::read_to_end()`].
    pub async fn read_to_end_checked(&mut self, buf: &mut Vec<u8>, entry: &ZipEntry) -> Result<usize> {
        self.0.read_to_end_checked(buf, entry).await
    }

    /// Reads all bytes until EOF has been reached, placing them into buf, and verifies the CRC32 values.
    ///
    /// This is a helper function synonymous to [`AsyncReadExt::read_to_string()`].
    pub async fn read_to_string_checked(&mut self, buf: &mut String, entry: &ZipEntry) -> Result<usize> {
        self.0.read_to_string_checked(buf, entry).await
    }

    /// Consumes this reader and returns the inner value.
    pub(crate) fn into_inner(self) -> R {
        self.0.into_inner().into_inner()
    }
}
