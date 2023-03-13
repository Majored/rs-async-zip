// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::{
    base::write::{EntryStreamWriter, ZipFileWriter as BaseZipFileWriter},
    error::Result,
    ZipEntry,
};

use tokio::io::AsyncWrite;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

// TODO: Remove exposed Compat wrappers from public API.

pub struct ZipFileWriter<W: AsyncWrite + Unpin>(BaseZipFileWriter<Compat<W>>);

impl<W: AsyncWrite + Unpin> ZipFileWriter<W> {
    /// Construct a new ZIP file writer from a mutable reference to a writer.
    pub fn new(writer: W) -> Self {
        Self(BaseZipFileWriter::new(writer.compat_write()))
    }

    /// Force the ZIP writer to operate in non-ZIP64 mode.
    /// If any files would need ZIP64, an error will be raised.
    pub fn force_no_zip64(self) -> Self {
        Self(self.0.force_no_zip64())
    }

    /// Force the ZIP writer to emit Zip64 structs at the end of the archive.
    /// Zip64 extended fields will only be written if needed.
    pub fn force_zip64(self) -> Self {
        Self(self.0.force_zip64())
    }

    /// Write a new ZIP entry of known size and data.
    pub async fn write_entry_whole<E: Into<ZipEntry>>(&mut self, entry: E, data: &[u8]) -> Result<()> {
        self.0.write_entry_whole(entry, data).await
    }

    /// Write an entry of unknown size and data via streaming (ie. using a data descriptor).
    /// The generated Local File Header will be invalid, with no compressed size, uncompressed size,
    /// and a null CRC. This might cause problems with the destination reader.
    pub async fn write_entry_stream<E: Into<ZipEntry>>(
        &mut self,
        entry: E,
    ) -> Result<EntryStreamWriter<'_, Compat<W>>> {
        Ok(self.0.write_entry_stream(entry).await?)
    }

    /// Set the ZIP file comment.
    pub fn comment(&mut self, comment: String) {
        self.0.comment(comment);
    }

    /// Returns a mutable reference to the inner writer.
    ///
    /// Care should be taken when using this inner writer as doing so may invalidate internal state of this writer.
    pub fn inner_mut(&mut self) -> &mut W {
        self.0.inner_mut().get_mut()
    }

    /// Consumes this ZIP writer and completes all closing tasks.
    ///
    /// This includes:
    /// - Writing all central directory headers.
    /// - Writing the end of central directory header.
    /// - Writing the file comment.
    ///
    /// Failure to call this function before going out of scope would result in a corrupted ZIP file.
    pub async fn close(self) -> Result<W> {
        Ok(self.0.close().await?.into_inner())
    }
}
