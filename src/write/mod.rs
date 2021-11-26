// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports writing ZIP files (unimplemented).

use crate::error::Result;
use crate::Compression;
use crate::header::CentralDirectoryHeader;

use std::io::Cursor;

use tokio::io::AsyncWriteExt;
use async_compression::tokio::write::DeflateEncoder;
use tokio::io::AsyncWrite;

pub struct EntryOptions<'a> {
    filename: &'a str,
    compression: Compression,
}

impl<'a> EntryOptions<'a> {
    pub fn new(filename: &'a str, compression: Compression) -> Self {
        Self { filename, compression }
    }
}

pub struct ZipFileWriter<'a, W: AsyncWrite + Unpin> {
    writer: &'a mut W,
    cd_entries: Vec<CentralDirectoryHeader>,
}

impl<'a, W: AsyncWrite + Unpin> ZipFileWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer, cd_entries: Vec::new() }
    }

    pub async fn write_entry(&mut self, opts: EntryOptions<'_>, data: &[u8]) -> Result<()> {
        if let Compression::Stored = &opts.compression {
            // Handle stored separately so we don't need to heap-allocate anything.
        }

        let compressed_data = compress(&opts.compression, data).await;

        unimplemented!();
    }

    pub fn stream_write_entry(&mut self, opts: EntryOptions) -> Result<()> {
        unimplemented!();
    }

    pub fn close(self) -> Result<()> {
        unimplemented!();
    }
}

async fn compress(compression: &Compression, data: &[u8]) -> Vec<u8> {
    match compression {
        Compression::Deflate => {
            // TODO: Reduce reallocations of Vec by making a lower-bound estimate of the length reduction and
            // pre-initialising the Vec to that length. Then truncate() to the actual number of bytes written.
            let mut writer = DeflateEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.into_inner().into_inner()
        }
        _ => {
            unimplemented!();
        }
    }
}