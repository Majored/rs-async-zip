// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module for reading ZIP file entries concurrently from a seekable source (synchronised over the underlying src).
//!
//! # Note
//! This module is unimplemented, and calls to ZipFileReader::new() will panic. Whilst I haven't put much thought into
//! impl, synchronising over a single seekable source creates a lot of challenges. Each call to read will have to do a
//! preemptive seek to the entry's data offset, and concurrent seeks can't interfere with each other. Thus, if using a
//! locking approach, we may have to hold the lock from the start of seeking to the end of reading.
//!
//! An async impl creates even more challenges as we have no guarantee when or even if a future (async seek or read)
//! will complete, thus we may create a deadlock.
//!
//! Feel free to open an issue/PR if you have a good approach for this.

use crate::error::{Result, ZipError};
use crate::read::{CompressionReader, ZipEntry, ZipEntryReader};

use std::io::SeekFrom;
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, ReadBuf};

/// A reader which acts concurrently over an in-memory buffer.
pub struct ZipFileReader<R: AsyncRead + AsyncSeek + Unpin> {
    pub(crate) reader: Arc<Mutex<R>>,
    pub(crate) entries: Vec<ZipEntry>,
}

#[allow(unreachable_code, unused_variables)]
impl<R: AsyncRead + AsyncSeek + Unpin> ZipFileReader<R> {
    /// Constructs a new ZIP file reader from an in-memory buffer.
    pub async fn new(reader: R) -> Result<ZipFileReader<R>> {
        unimplemented!();

        let entries = crate::read::seek::read_cd(&mut reader).await?;
        Ok(ZipFileReader {
            reader: Arc::new(Mutex::new(reader)),
            entries,
        })
    }

    crate::read::reader_entry_impl!();

    /// Opens an entry at the provided index for reading.
    pub async fn entry_reader<'a>(&'a self, index: usize) -> Result<ZipEntryReader<'a, GuardedReader<R>>> {
        let entry = self.entries.get(index).ok_or(ZipError::EntryIndexOutOfBounds)?;

        let mut guarded_reader = GuardedReader {
            reader: self.reader.clone(),
        };
        guarded_reader.seek(SeekFrom::Start(entry.data_offset())).await?;
        let reader = CompressionReader::from_reader(entry.compression(), guarded_reader);

        Ok(ZipEntryReader { entry, reader })
    }
}

#[derive(Clone)]
pub struct GuardedReader<R: AsyncRead + AsyncSeek + Unpin> {
    pub(crate) reader: Arc<Mutex<R>>,
}

impl<R: AsyncRead + AsyncSeek + Unpin> AsyncSeek for GuardedReader<R> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> tokio::io::Result<()> {
        Pin::new(self.reader.lock().unwrap().deref_mut()).start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<tokio::io::Result<u64>> {
        Pin::new(self.reader.lock().unwrap().deref_mut()).poll_complete(cx)
    }
}

impl<R: AsyncRead + AsyncSeek + Unpin> AsyncRead for GuardedReader<R> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        Pin::new(self.reader.lock().unwrap().deref_mut()).poll_read(cx, buf)
    }
}
