// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports writing ZIP files.

pub mod entry_stream;
pub(crate) mod entry_whole;
pub(crate) mod offset_writer;
pub(crate) mod compressed_writer;

use crate::error::Result;
use crate::header::{CentralDirectoryHeader, EndOfCentralDirectoryHeader};
use crate::Compression;
use entry_stream::EntryStreamWriter;
use entry_whole::EntryWholeWriter;
use offset_writer::OffsetAsyncWriter;

use tokio::io::{AsyncWrite, AsyncWriteExt};

/// A set of options for opening new ZIP entries.
pub struct EntryOptions {
    filename: String,
    compression: Compression,
    extra: Vec<u8>,
    comment: String,
}

impl EntryOptions {
    /// Construct a new set of options from its required constituents.
    pub fn new(filename: String, compression: Compression) -> Self {
        EntryOptions { filename, compression, extra: Vec::new(), comment: String::new() }
    }
    
    /// Consume the options and override the extra field data.
    pub fn extra(mut self, extra: Vec<u8>) -> Self {
        self.extra = extra;
        self
    }

    /// Consume the options and override the file comment.
    pub fn comment(mut self, comment: String) -> Self {
        self.comment = comment;
        self
    }
}

pub(crate) struct CentralDirectoryEntry {
    pub header: CentralDirectoryHeader,
    pub opts: EntryOptions,
}

/// A writer which acts over a non-seekable source.
pub struct ZipFileWriter<'a, W: AsyncWrite + Unpin> {
    pub(crate) writer: OffsetAsyncWriter<&'a mut W>,
    pub(crate) cd_entries: Vec<CentralDirectoryEntry>,
}

impl<'a, W: AsyncWrite + Unpin> ZipFileWriter<'a, W> {
    /// Construct a new ZIP file writer from a mutable reference to a writer.
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer: OffsetAsyncWriter::from_raw(writer),
            cd_entries: Vec::new(),
        }
    }

    /// Write a new ZIP entry of known size and data.
    pub async fn write_entry_whole(&mut self, options: EntryOptions, data: &[u8]) -> Result<()> {
        EntryWholeWriter::from_raw(self, options, data).write().await
    }

    /// Write an entry of unknown size and data via streaming (ie. using a data descriptor).
    pub async fn write_entry_stream<'b>(&'b mut self, options: EntryOptions) -> Result<EntryStreamWriter<'a, 'b, W>> {
        EntryStreamWriter::from_raw(self, options).await
    }

    /// Close the ZIP file by writing all central directory headers.
    pub async fn close(mut self) -> Result<()> {
        let cd_offset = self.writer.offset();

        for entry in &self.cd_entries {
            self.writer.write(&crate::delim::CDFHD.to_le_bytes()).await?;
            self.writer.write(&entry.header.to_slice()).await?;
            self.writer.write(entry.opts.filename.as_bytes()).await?;
            self.writer.write(&entry.opts.extra).await?;
            self.writer.write(entry.opts.comment.as_bytes()).await?;
        }

        let header = EndOfCentralDirectoryHeader {
            disk_num: 0,
            start_cent_dir_disk: 0,
            num_of_entries_disk: self.cd_entries.len() as u16,
            num_of_entries: self.cd_entries.len() as u16,
            size_cent_dir: (self.writer.offset() - cd_offset) as u32,
            cent_dir_offset: cd_offset as u32,
            file_comm_length: 0,
        };

        self.writer.write(&crate::delim::EOCDD.to_le_bytes()).await?;
        self.writer.write(&header.to_slice()).await?;

        Ok(())
    }
}
