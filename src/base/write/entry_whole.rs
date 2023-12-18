// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::base::write::get_or_put_info_zip_unicode_comment_extra_field_mut;
use crate::base::write::get_or_put_info_zip_unicode_path_extra_field_mut;
use crate::base::write::{CentralDirectoryEntry, ZipFileWriter};
use crate::entry::ZipEntry;
use crate::error::{Result, Zip64ErrorCase, ZipError};
use crate::spec::extra_field::Zip64ExtendedInformationExtraFieldBuilder;
use crate::spec::header::{InfoZipUnicodeCommentExtraField, InfoZipUnicodePathExtraField};
use crate::spec::{
    extra_field::ExtraFieldAsBytes,
    header::{CentralDirectoryRecord, ExtraField, GeneralPurposeFlag, LocalFileHeader},
    Compression,
};
use crate::StringEncoding;
#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use futures_util::io::Cursor;

use crate::spec::consts::{NON_ZIP64_MAX_NUM_FILES, NON_ZIP64_MAX_SIZE};
#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use async_compression::futures::write;
use futures_util::io::{AsyncWrite, AsyncWriteExt};

pub struct EntryWholeWriter<'b, 'c, W: AsyncWrite + Unpin> {
    writer: &'b mut ZipFileWriter<W>,
    entry: ZipEntry,
    data: &'c [u8],
}

impl<'b, 'c, W: AsyncWrite + Unpin> EntryWholeWriter<'b, 'c, W> {
    pub fn from_raw(writer: &'b mut ZipFileWriter<W>, entry: ZipEntry, data: &'c [u8]) -> Self {
        Self { writer, entry, data }
    }

    pub async fn write(mut self) -> Result<()> {
        let mut _compressed_data: Option<Vec<u8>> = None;
        let compressed_data = match self.entry.compression() {
            Compression::Stored => self.data,
            #[cfg(any(
                feature = "deflate",
                feature = "bzip2",
                feature = "zstd",
                feature = "lzma",
                feature = "xz",
                feature = "deflate64"
            ))]
            _ => {
                _compressed_data =
                    Some(compress(self.entry.compression(), self.data, self.entry.compression_level).await);
                _compressed_data.as_ref().unwrap()
            }
        };

        let mut zip64_extra_field_builder = None;

        let (lfh_uncompressed_size, lfh_compressed_size) = if self.data.len() as u64 > NON_ZIP64_MAX_SIZE as u64
            || compressed_data.len() as u64 > NON_ZIP64_MAX_SIZE as u64
        {
            if self.writer.force_no_zip64 {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::LargeFile));
            }
            if !self.writer.is_zip64 {
                self.writer.is_zip64 = true;
            }
            zip64_extra_field_builder = Some(
                Zip64ExtendedInformationExtraFieldBuilder::new()
                    .sizes(compressed_data.len() as u64, self.data.len() as u64),
            );
            (NON_ZIP64_MAX_SIZE, NON_ZIP64_MAX_SIZE)
        } else {
            (self.data.len() as u32, compressed_data.len() as u32)
        };

        let lh_offset = if self.writer.writer.offset() > NON_ZIP64_MAX_SIZE as u64 {
            if self.writer.force_no_zip64 {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::LargeFile));
            }
            if !self.writer.is_zip64 {
                self.writer.is_zip64 = true;
            }

            if let Some(zip64_extra_field) = zip64_extra_field_builder {
                zip64_extra_field_builder = Some(zip64_extra_field.relative_header_offset(self.writer.writer.offset()));
            } else {
                zip64_extra_field_builder = Some(
                    Zip64ExtendedInformationExtraFieldBuilder::new()
                        .relative_header_offset(self.writer.writer.offset()),
                );
            }
            NON_ZIP64_MAX_SIZE
        } else {
            self.writer.writer.offset() as u32
        };

        if let Some(builder) = zip64_extra_field_builder {
            if !builder.eof_only() {
                self.entry.extra_fields.push(ExtraField::Zip64ExtendedInformation(builder.build()?));
                zip64_extra_field_builder = None;
            } else {
                zip64_extra_field_builder = Some(builder);
            }
        }

        let utf8_without_alternative =
            self.entry.filename().is_utf8_without_alternative() && self.entry.comment().is_utf8_without_alternative();
        if !utf8_without_alternative {
            if matches!(self.entry.filename().encoding(), StringEncoding::Utf8) {
                let u_file_name = self.entry.filename().as_bytes().to_vec();
                if !u_file_name.is_empty() {
                    let basic_crc32 = crc32fast::hash(
                        self.entry.filename().alternative().unwrap_or_else(|| self.entry.filename().as_bytes()),
                    );
                    let upath_field =
                        get_or_put_info_zip_unicode_path_extra_field_mut(self.entry.extra_fields.as_mut());
                    if let InfoZipUnicodePathExtraField::V1 { crc32, unicode } = upath_field {
                        *crc32 = basic_crc32;
                        *unicode = u_file_name;
                    }
                }
            }
            if matches!(self.entry.comment().encoding(), StringEncoding::Utf8) {
                let u_comment = self.entry.comment().as_bytes().to_vec();
                if !u_comment.is_empty() {
                    let basic_crc32 = crc32fast::hash(
                        self.entry.comment().alternative().unwrap_or_else(|| self.entry.comment().as_bytes()),
                    );
                    let ucom_field =
                        get_or_put_info_zip_unicode_comment_extra_field_mut(self.entry.extra_fields.as_mut());
                    if let InfoZipUnicodeCommentExtraField::V1 { crc32, unicode } = ucom_field {
                        *crc32 = basic_crc32;
                        *unicode = u_comment;
                    }
                }
            }
        }

        let filename_basic = self.entry.filename().alternative().unwrap_or_else(|| self.entry.filename().as_bytes());
        let comment_basic = self.entry.comment().alternative().unwrap_or_else(|| self.entry.comment().as_bytes());

        let lf_header = LocalFileHeader {
            compressed_size: lfh_compressed_size,
            uncompressed_size: lfh_uncompressed_size,
            compression: self.entry.compression().into(),
            crc: crc32fast::hash(self.data),
            extra_field_length: self
                .entry
                .extra_fields()
                .count_bytes()
                .try_into()
                .map_err(|_| ZipError::ExtraFieldTooLarge)?,
            file_name_length: filename_basic.len().try_into().map_err(|_| ZipError::FileNameTooLarge)?,
            mod_time: self.entry.last_modification_date().time,
            mod_date: self.entry.last_modification_date().date,
            version: crate::spec::version::as_needed_to_extract(&self.entry),
            flags: GeneralPurposeFlag {
                data_descriptor: false,
                encrypted: false,
                filename_unicode: utf8_without_alternative,
            },
        };

        let mut header = CentralDirectoryRecord {
            v_made_by: crate::spec::version::as_made_by(),
            v_needed: lf_header.version,
            compressed_size: lf_header.compressed_size,
            uncompressed_size: lf_header.uncompressed_size,
            compression: lf_header.compression,
            crc: lf_header.crc,
            extra_field_length: lf_header.extra_field_length,
            file_name_length: lf_header.file_name_length,
            file_comment_length: comment_basic.len().try_into().map_err(|_| ZipError::CommentTooLarge)?,
            mod_time: lf_header.mod_time,
            mod_date: lf_header.mod_date,
            flags: lf_header.flags,
            disk_start: 0,
            inter_attr: self.entry.internal_file_attribute(),
            exter_attr: self.entry.external_file_attribute(),
            lh_offset,
        };

        self.writer.writer.write_all(&crate::spec::consts::LFH_SIGNATURE.to_le_bytes()).await?;
        self.writer.writer.write_all(&lf_header.as_slice()).await?;
        self.writer.writer.write_all(filename_basic).await?;
        self.writer.writer.write_all(&self.entry.extra_fields().as_bytes()).await?;
        self.writer.writer.write_all(compressed_data).await?;

        if let Some(builder) = zip64_extra_field_builder {
            self.entry.extra_fields.push(ExtraField::Zip64ExtendedInformation(builder.build()?));
            header.extra_field_length =
                self.entry.extra_fields().count_bytes().try_into().map_err(|_| ZipError::ExtraFieldTooLarge)?;
        }

        self.writer.cd_entries.push(CentralDirectoryEntry { header, entry: self.entry });
        // Ensure that we can fit this many files in this archive if forcing no zip64
        if self.writer.cd_entries.len() > NON_ZIP64_MAX_NUM_FILES as usize {
            if self.writer.force_no_zip64 {
                return Err(ZipError::Zip64Needed(Zip64ErrorCase::TooManyFiles));
            }
            if !self.writer.is_zip64 {
                self.writer.is_zip64 = true;
            }
        }
        Ok(())
    }
}

#[cfg(any(
    feature = "deflate",
    feature = "bzip2",
    feature = "zstd",
    feature = "lzma",
    feature = "xz",
    feature = "deflate64"
))]
async fn compress(compression: Compression, data: &[u8], level: async_compression::Level) -> Vec<u8> {
    // TODO: Reduce reallocations of Vec by making a lower-bound estimate of the length reduction and
    // pre-initialising the Vec to that length. Then truncate() to the actual number of bytes written.
    match compression {
        #[cfg(feature = "deflate")]
        Compression::Deflate => {
            let mut writer = write::DeflateEncoder::with_quality(Cursor::new(Vec::new()), level);
            writer.write_all(data).await.unwrap();
            writer.close().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "deflate64")]
        Compression::Deflate64 => panic!("compressing deflate64 is not supported"),
        #[cfg(feature = "bzip2")]
        Compression::Bz => {
            let mut writer = write::BzEncoder::with_quality(Cursor::new(Vec::new()), level);
            writer.write_all(data).await.unwrap();
            writer.close().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "lzma")]
        Compression::Lzma => {
            let mut writer = write::LzmaEncoder::with_quality(Cursor::new(Vec::new()), level);
            writer.write_all(data).await.unwrap();
            writer.close().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "xz")]
        Compression::Xz => {
            let mut writer = write::XzEncoder::with_quality(Cursor::new(Vec::new()), level);
            writer.write_all(data).await.unwrap();
            writer.close().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "zstd")]
        Compression::Zstd => {
            let mut writer = write::ZstdEncoder::with_quality(Cursor::new(Vec::new()), level);
            writer.write_all(data).await.unwrap();
            writer.close().await.unwrap();
            writer.into_inner().into_inner()
        }
        _ => unreachable!(),
    }
}
