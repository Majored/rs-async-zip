// Copyright (c) 2021-2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports writing ZIP files.
//!
//! # Example
//! ### Whole data (u8 slice)
//! ```no_run
//! # #[cfg(feature = "deflate")]
//! # {
//! # use async_zip::{Compression, ZipEntryBuilder, write::ZipFileWriter};
//! # use tokio::{fs::File, io::AsyncWriteExt};
//! # use async_zip::error::ZipError;
//! #
//! # async fn run() -> Result<(), ZipError> {
//! let mut file = File::create("foo.zip").await?;
//! let mut writer = ZipFileWriter::new(&mut file);
//!
//! let data = b"This is an example file.";
//! let opts = ZipEntryBuilder::new(String::from("foo.txt"), Compression::Deflate);
//!
//! writer.write_entry_whole(opts, data).await?;
//! writer.close().await?;
//! #   Ok(())
//! # }
//! # }
//! ```
//! ### Stream data (unknown size & data)
//! ```no_run
//! # #[cfg(feature = "deflate")]
//! # {
//! # use async_zip::{Compression, ZipEntryBuilder, write::ZipFileWriter};
//! # use tokio::{fs::File, io::AsyncWriteExt};
//! # use async_zip::error::ZipError;
//! #
//! # async fn run() -> Result<(), ZipError> {
//! let mut file = File::create("foo.zip").await?;
//! let mut writer = ZipFileWriter::new(&mut file);
//!
//! let data = b"This is an example file.";
//! let opts = ZipEntryBuilder::new(String::from("bar.txt"), Compression::Deflate);
//!
//! let mut entry_writer = writer.write_entry_stream(opts).await?;
//! entry_writer.write_all(data).await.unwrap();
//!
//! entry_writer.close().await?;
//! writer.close().await?;
//! #   Ok(())
//! # }
//! # }
//! ```

pub(crate) mod compressed_writer;
pub(crate) mod entry_stream;
pub(crate) mod entry_whole;
pub(crate) mod io;

pub use entry_stream::EntryStreamWriter;

use crate::entry::ZipEntry;
use crate::error::Result;
use crate::spec::header::{CentralDirectoryRecord, EndOfCentralDirectoryHeader};
use entry_whole::EntryWholeWriter;
use io::offset::AsyncOffsetWriter;

use tokio::io::{AsyncWrite, AsyncWriteExt};

pub(crate) struct CentralDirectoryEntry {
    pub header: CentralDirectoryRecord,
    pub entry: ZipEntry,
}

/// A ZIP file writer which acts over AsyncWrite implementers.
///
/// # Note
/// - [`ZipFileWriter::close()`] must be called before a stream writer goes out of scope.
pub struct ZipFileWriter<W: AsyncWrite + Unpin> {
    pub(crate) writer: AsyncOffsetWriter<W>,
    pub(crate) cd_entries: Vec<CentralDirectoryEntry>,
    comment_opt: Option<String>,
}

impl<W: AsyncWrite + Unpin> ZipFileWriter<W> {
    /// Construct a new ZIP file writer from a mutable reference to a writer.
    pub fn new(writer: W) -> Self {
        Self { writer: AsyncOffsetWriter::new(writer), cd_entries: Vec::new(), comment_opt: None }
    }

    /// Write a new ZIP entry of known size and data.
    pub async fn write_entry_whole<E: Into<ZipEntry>>(&mut self, entry: E, data: &[u8]) -> Result<()> {
        EntryWholeWriter::from_raw(self, entry.into(), data).write().await
    }

    /// Write an entry of unknown size and data via streaming (ie. using a data descriptor).
    pub async fn write_entry_stream<E: Into<ZipEntry>>(&mut self, entry: E) -> Result<EntryStreamWriter<'_, W>> {
        EntryStreamWriter::from_raw(self, entry.into()).await
    }

    /// Set the ZIP file comment.
    pub fn comment(&mut self, comment: String) {
        self.comment_opt = Some(comment);
    }

    pub fn inner_mut(&mut self) -> &mut W {
	self.writer.inner_mut()
    }

    /// Consumes this ZIP writer and completes all closing tasks.
    ///
    /// This includes:
    /// - Writing all central directroy headers.
    /// - Writing the end of central directory header.
    /// - Writing the file comment.
    ///
    /// Failiure to call this function before going out of scope would result in a corrupted ZIP file.
    pub async fn close(mut self) -> Result<W> {
        let cd_offset = self.writer.offset();

        for entry in &self.cd_entries {
            self.writer.write_all(&crate::spec::consts::CDH_SIGNATURE.to_le_bytes()).await?;
            self.writer.write_all(&entry.header.as_slice()).await?;
            self.writer.write_all(entry.entry.filename().as_bytes()).await?;
            self.writer.write_all(entry.entry.extra_field()).await?;
            self.writer.write_all(entry.entry.comment().as_bytes()).await?;
        }

        let header = EndOfCentralDirectoryHeader {
            disk_num: 0,
            start_cent_dir_disk: 0,
            num_of_entries_disk: self.cd_entries.len() as u16,
            num_of_entries: self.cd_entries.len() as u16,
            size_cent_dir: (self.writer.offset() - cd_offset) as u32,
            cent_dir_offset: cd_offset as u32,
            file_comm_length: self.comment_opt.as_ref().map(|v| v.len() as u16).unwrap_or_default(),
        };

        self.writer.write_all(&crate::spec::consts::EOCDR_SIGNATURE.to_le_bytes()).await?;
        self.writer.write_all(&header.as_slice()).await?;
        if let Some(comment) = self.comment_opt {
            self.writer.write_all(comment.as_bytes()).await?;
        }

        Ok(self.writer.into_inner())
    }
}
