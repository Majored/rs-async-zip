// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::base::write::compressed_writer::CompressedAsyncWriter;
use crate::base::write::io::offset::AsyncOffsetWriter;
use crate::base::write::CentralDirectoryEntry;
use crate::base::write::ZipFileWriter;
use crate::entry::ZipEntry;
use crate::error::{Result, Zip64ErrorCase, ZipError};
use crate::spec::extra_field::ExtraFieldAsBytes;
use crate::spec::header::{
    CentralDirectoryRecord, ExtraField, GeneralPurposeFlag, HeaderId, LocalFileHeader,
    Zip64ExtendedInformationExtraField,
};

use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::base::read::get_zip64_extra_field_mut;
use crate::spec::consts::{NON_ZIP64_MAX_NUM_FILES, NON_ZIP64_MAX_SIZE};
use crc32fast::Hasher;
use futures_util::io::{AsyncWrite, AsyncWriteExt};

/// An entry writer which supports the streaming of data (ie. the writing of unknown size or data at runtime).
///
/// # Note
/// - This writer cannot be manually constructed; instead, use [`ZipFileWriter::write_entry_stream()`].
/// - [`EntryStreamWriter::close()`] must be called before a stream writer goes out of scope.
/// - Utilities for working with [`AsyncWrite`] values are provided by [`AsyncWriteExt`].
pub struct EntryStreamWriter<'b, W: AsyncWrite + Unpin> {
    writer: AsyncOffsetWriter<CompressedAsyncWriter<'b, W>>,
    cd_entries: &'b mut Vec<CentralDirectoryEntry>,
    entry: ZipEntry,
    hasher: Hasher,
    lfh: LocalFileHeader,
    lfh_offset: usize,
    data_offset: usize,
    force_no_zip64: bool,
    /// To write back to the original writer if zip64 is required.
    is_zip64: &'b mut bool,
}

impl<'b, W: AsyncWrite + Unpin> EntryStreamWriter<'b, W> {
    pub(crate) async fn from_raw(
        writer: &'b mut ZipFileWriter<W>,
        mut entry: ZipEntry,
    ) -> Result<EntryStreamWriter<'b, W>> {
        let lfh_offset = writer.writer.offset();
        let lfh = EntryStreamWriter::write_lfh(writer, &mut entry).await?;
        let data_offset = writer.writer.offset();
        let force_no_zip64 = writer.force_no_zip64;

        let cd_entries = &mut writer.cd_entries;
        let is_zip64 = &mut writer.is_zip64;
        let writer = AsyncOffsetWriter::new(CompressedAsyncWriter::from_raw(&mut writer.writer, entry.compression()));

        Ok(EntryStreamWriter {
            writer,
            cd_entries,
            entry,
            lfh,
            lfh_offset,
            data_offset,
            hasher: Hasher::new(),
            force_no_zip64,
            is_zip64,
        })
    }

    async fn write_lfh(writer: &'b mut ZipFileWriter<W>, entry: &mut ZipEntry) -> Result<LocalFileHeader> {
        // Always emit a zip64 extended field, even if we don't need it, because we *might* need it.
        // If we are forcing no zip, we will have to error later if the file is too large.
        let (lfh_compressed, lfh_uncompressed) = if !writer.force_no_zip64 {
            if !writer.is_zip64 {
                writer.is_zip64 = true;
            }
            entry.extra_fields.push(ExtraField::Zip64ExtendedInformationExtraField(
                Zip64ExtendedInformationExtraField {
                    header_id: HeaderId::Zip64ExtendedInformationExtraField,
                    data_size: 16,
                    uncompressed_size: entry.uncompressed_size,
                    compressed_size: entry.compressed_size,
                    relative_header_offset: None,
                    disk_start_number: None,
                },
            ));

            (NON_ZIP64_MAX_SIZE, NON_ZIP64_MAX_SIZE)
        } else {
            if entry.compressed_size > NON_ZIP64_MAX_SIZE as u64 || entry.uncompressed_size > NON_ZIP64_MAX_SIZE as u64
            {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::LargeFile));
            }

            (entry.compressed_size as u32, entry.uncompressed_size as u32)
        };

        let lfh = LocalFileHeader {
            compressed_size: lfh_compressed,
            uncompressed_size: lfh_uncompressed,
            compression: entry.compression().into(),
            crc: entry.crc32,
            extra_field_length: entry
                .extra_fields()
                .count_bytes()
                .try_into()
                .map_err(|_| ZipError::ExtraFieldTooLarge)?,
            file_name_length: entry.filename().as_bytes().len().try_into().map_err(|_| ZipError::FileNameTooLarge)?,
            mod_time: entry.last_modification_date().time,
            mod_date: entry.last_modification_date().date,
            version: crate::spec::version::as_needed_to_extract(entry),
            flags: GeneralPurposeFlag {
                data_descriptor: true,
                encrypted: false,
                // TODO: resolve unwrap use
                filename_unicode: !entry.filename().as_str().unwrap().is_ascii(),
            },
        };

        writer.writer.write_all(&crate::spec::consts::LFH_SIGNATURE.to_le_bytes()).await?;
        writer.writer.write_all(&lfh.as_slice()).await?;
        writer.writer.write_all(entry.filename().as_bytes()).await?;
        writer.writer.write_all(&entry.extra_fields().as_bytes()).await?;

        Ok(lfh)
    }

    /// Consumes this entry writer and completes all closing tasks.
    ///
    /// This includes:
    /// - Finalising the CRC32 hash value for the written data.
    /// - Calculating the compressed and uncompressed byte sizes.
    /// - Constructing a central directory header.
    /// - Pushing that central directory header to the [`ZipFileWriter`]'s store.
    ///
    /// Failure to call this function before going out of scope would result in a corrupted ZIP file.
    pub async fn close(mut self) -> Result<()> {
        self.writer.close().await?;

        let crc = self.hasher.finalize();
        let uncompressed_size = self.writer.offset() as u64;
        let inner_writer = self.writer.into_inner().into_inner();
        let compressed_size = (inner_writer.offset() - self.data_offset) as u64;

        let (cdr_compressed_size, cdr_uncompressed_size) = if self.force_no_zip64 {
            if uncompressed_size > NON_ZIP64_MAX_SIZE as u64 || compressed_size > NON_ZIP64_MAX_SIZE as u64 {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::LargeFile));
            }
            (uncompressed_size as u32, compressed_size as u32)
        } else {
            // When streaming an entry, we are always using a zip64 field.
            match get_zip64_extra_field_mut(&mut self.entry.extra_fields) {
                // This case shouldn't be necessary but is included for completeness.
                None => {
                    self.entry.extra_fields.push(ExtraField::Zip64ExtendedInformationExtraField(
                        Zip64ExtendedInformationExtraField {
                            header_id: HeaderId::Zip64ExtendedInformationExtraField,
                            data_size: 16,
                            uncompressed_size,
                            compressed_size,
                            relative_header_offset: None,
                            disk_start_number: None,
                        },
                    ));
                    self.lfh.extra_field_length =
                        self.entry.extra_fields().count_bytes().try_into().map_err(|_| ZipError::ExtraFieldTooLarge)?;
                }
                Some(zip64) => {
                    zip64.uncompressed_size = uncompressed_size;
                    zip64.compressed_size = compressed_size;
                }
            }

            (NON_ZIP64_MAX_SIZE, NON_ZIP64_MAX_SIZE)
        };

        inner_writer.write_all(&crate::spec::consts::DATA_DESCRIPTOR_SIGNATURE.to_le_bytes()).await?;
        inner_writer.write_all(&crc.to_le_bytes()).await?;
        inner_writer.write_all(&cdr_compressed_size.to_le_bytes()).await?;
        inner_writer.write_all(&cdr_uncompressed_size.to_le_bytes()).await?;

        let cdh = CentralDirectoryRecord {
            compressed_size: cdr_compressed_size,
            uncompressed_size: cdr_uncompressed_size,
            crc,
            v_made_by: crate::spec::version::as_made_by(),
            v_needed: self.lfh.version,
            compression: self.lfh.compression,
            extra_field_length: self.lfh.extra_field_length,
            file_name_length: self.lfh.file_name_length,
            // TODO: resolve unwrap usage
            file_comment_length: self
                .entry
                .comment()
                .as_str()
                .unwrap()
                .len()
                .try_into()
                .map_err(|_| ZipError::CommentTooLarge)?,
            mod_time: self.lfh.mod_time,
            mod_date: self.lfh.mod_date,
            flags: self.lfh.flags,
            disk_start: 0,
            inter_attr: self.entry.internal_file_attribute(),
            exter_attr: self.entry.external_file_attribute(),
            lh_offset: self.lfh_offset as u32,
        };

        self.cd_entries.push(CentralDirectoryEntry { header: cdh, entry: self.entry });
        // Ensure that we can fit this many files in this archive if forcing no zip64
        if self.cd_entries.len() > NON_ZIP64_MAX_NUM_FILES as usize {
            if self.force_no_zip64 {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::TooManyFiles));
            }
            if !*self.is_zip64 {
                *self.is_zip64 = true;
            }
        }

        Ok(())
    }
}

impl<'a, W: AsyncWrite + Unpin> AsyncWrite for EntryStreamWriter<'a, W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        let poll = Pin::new(&mut self.writer).poll_write(cx, buf);

        if let Poll::Ready(Ok(written)) = poll {
            self.hasher.update(&buf[0..written]);
        }

        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.writer).poll_close(cx)
    }
}
