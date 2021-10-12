// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from an in-memory buffer.

use crate::error::{Result, ZipError};
use crate::read::{ZipEntry, ZipEntryReader, CompressionReader};

use std::pin::Pin;
use std::io::SeekFrom;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, ReadBuf};

/// The type returned as an entry reader within this concurrent module.
pub type ConcurrentReader<'b, 'a> = ZipEntryReader<'b, AsyncCursor<'a>>;

/// A reader which acts concurrently over an in-memory buffer.
pub struct ZipFileReader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) entries: Vec<ZipEntry>,
}

impl<'a> ZipFileReader<'a> {
    /// Constructs a new ZIP file reader from an in-memory buffer.
    pub async fn new(data: &'a [u8]) -> Result<ZipFileReader<'a>> {
        let entries = crate::read::seek::read_cd(&mut AsyncCursor::new(data)).await?;
        Ok(ZipFileReader { data, entries })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader<'b>(&'b mut self, index: usize) -> Result<ConcurrentReader<'b, 'a>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;
        let mut cursor = AsyncCursor::new(self.data.clone());

        cursor.seek(SeekFrom::Start(entry.data_offset())).await?;
        let reader = CompressionReader::from_reader(entry.compression(), cursor);

        Ok(ZipEntryReader { entry, reader })
    }
}

/// A async cursor over a slice of data.
pub struct AsyncCursor<'a> {
    data: &'a [u8],
    read: usize,
}

impl<'a> AsyncCursor<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        AsyncCursor { data, read: 0 }
    }
}

impl<'a> AsyncRead for AsyncCursor<'a> {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        let mut data_remaining = self.data.len() - self.read;

        if data_remaining == 0 {
            return Poll::Ready(tokio::io::Result::Ok(()));
        } else if buf.remaining() < data_remaining {
            data_remaining = buf.remaining();
        }

        let upper_index = self.read + data_remaining;
        buf.put_slice(&self.data[self.read..upper_index]);
        self.read += data_remaining;

        Poll::Ready(tokio::io::Result::Ok(()))
    }
}

impl<'a> AsyncSeek for AsyncCursor<'a> {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> tokio::io::Result<()> {
        match position {
            SeekFrom::Start(inner) => self.read = inner as usize + 1,
            SeekFrom::End(inner) => self.read = (self.data.len() as i64 + inner - 1) as usize,
            SeekFrom::Current(inner) => self.read = (self.read as i64 + inner) as usize,
        };

        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<tokio::io::Result<u64>> {
        Poll::Ready(Ok(self.read as u64 - 1))
    }
}