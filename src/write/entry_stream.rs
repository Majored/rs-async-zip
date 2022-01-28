// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::write::{ZipFileWriter, EntryOptions};
use crate::error::Result;

use std::io::Error;
use std::pin::Pin;
use std::task::{Poll, Context};

use crc32fast::Hasher;
use tokio::io::{AsyncWrite};

// Taking a mutable reference ensures that no two writers can act upon the same ZipFileWriter.
pub struct EntryStreamWriter<'a, 'brw, W: AsyncWrite + Unpin> {
    raw_writer: &'brw mut ZipFileWriter<'a, W>,
    options: EntryOptions,
    hasher: Hasher,
    closed: bool,
}

impl<'a, 'brw, W: AsyncWrite + Unpin> EntryStreamWriter<'a, 'brw, W> {
    pub async fn from_raw(raw_writer: &'brw mut ZipFileWriter<'a, W>, options: EntryOptions) -> Result<EntryStreamWriter<'a, 'brw, W>> {
        let writer = EntryStreamWriter {
            raw_writer,
            options,
            hasher: Hasher::new(),
            closed: false,
        };

        // TODO: write LFH.

        Ok(writer)
    }

    pub async fn close(self) {
        unimplemented!();
    }
}

impl<'a, 'brw, W: AsyncWrite + Unpin> AsyncWrite for EntryStreamWriter<'a, 'brw, W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<std::result::Result<usize, Error>> {
        unimplemented!();
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        unimplemented!();
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<std::result::Result<(), Error>> {
        unimplemented!();
    }
}

impl<'a, 'brw, W: AsyncWrite + Unpin> Drop for EntryStreamWriter<'a, 'brw, W> {
    fn drop(&mut self) {
        if !self.closed {
            panic!("An EntryStreamWriter must be closed before being dropped.");
        }
    }
}

