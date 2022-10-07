// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::Result;
use crate::spec::compression::Compression;
use crate::spec::header::{CentralDirectoryHeader, GeneralPurposeFlag, LocalFileHeader};
use crate::write::{CentralDirectoryEntry, ZipFileWriter};
use crate::entry::ZipEntry;

#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use std::io::Cursor;

#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
use async_compression::tokio::write;
use crc32fast::Hasher;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct EntryWholeWriter<'b, 'c, W: AsyncWrite + Unpin> {
    writer: &'b mut ZipFileWriter<W>,
    entry: ZipEntry,
    data: &'c [u8],
}

impl<'b, 'c, W: AsyncWrite + Unpin> EntryWholeWriter<'b, 'c, W> {
    pub fn from_raw(writer: &'b mut ZipFileWriter<W>, entry: ZipEntry, data: &'c [u8]) -> Self {
        Self { writer, entry, data }
    }

    pub async fn write(self) -> Result<()> {
        let mut _compressed_data: Option<Vec<u8>> = None;
        let compressed_data = match self.entry.compression() {
            Compression::Stored => self.data,
            #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
            _ => {
                _compressed_data = Some(compress(self.entry.compression(), self.data).await);
                _compressed_data.as_ref().unwrap()
            }
        };

        let (mod_time, mod_date) = crate::spec::date::chrono_to_zip_time(self.entry.last_modification_date());

        let lf_header = LocalFileHeader {
            compressed_size: compressed_data.len() as u32,
            uncompressed_size: self.data.len() as u32,
            compression: self.entry.compression().into(),
            crc: compute_crc(self.data),
            extra_field_length: self.entry.extra_field().len() as u16,
            file_name_length: self.entry.filename().as_bytes().len() as u16,
            mod_time,
            mod_date,
            version: crate::spec::version::as_needed_to_extract(&self.entry),
            flags: GeneralPurposeFlag {
                data_descriptor: false,
                encrypted: false,
                filename_unicode: !self.entry.filename().is_ascii(),
            },
        };

        let header = CentralDirectoryHeader {
            v_made_by: crate::spec::version::as_made_by(),
            v_needed: lf_header.version,
            compressed_size: lf_header.compressed_size,
            uncompressed_size: lf_header.uncompressed_size,
            compression: lf_header.compression,
            crc: lf_header.crc,
            extra_field_length: lf_header.extra_field_length,
            file_name_length: lf_header.file_name_length,
            file_comment_length: self.entry.comment().len() as u16,
            mod_time: lf_header.mod_time,
            mod_date: lf_header.mod_date,
            flags: lf_header.flags,
            disk_start: 0,
            inter_attr: self.entry.internal_file_attribute(),
            exter_attr: self.entry.external_file_attribute(),
            lh_offset: self.writer.writer.offset() as u32,
        };

        self.writer.writer.write_all(&crate::spec::signature::LOCAL_FILE_HEADER.to_le_bytes()).await?;
        self.writer.writer.write_all(&lf_header.as_slice()).await?;
        self.writer.writer.write_all(self.entry.filename().as_bytes()).await?;
        self.writer.writer.write_all(&self.entry.extra_field()).await?;
        self.writer.writer.write_all(compressed_data).await?;

        self.writer.cd_entries.push(CentralDirectoryEntry { header, entry: self.entry });

        Ok(())
    }
}

#[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
async fn compress(compression: Compression, data: &[u8]) -> Vec<u8> {
    // TODO: Reduce reallocations of Vec by making a lower-bound estimate of the length reduction and
    // pre-initialising the Vec to that length. Then truncate() to the actual number of bytes written.
    match compression {
        #[cfg(feature = "deflate")]
        Compression::Deflate => {
            let mut writer = write::DeflateEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "bzip2")]
        Compression::Bz => {
            let mut writer = write::BzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "lzma")]
        Compression::Lzma => {
            let mut writer = write::LzmaEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "xz")]
        Compression::Xz => {
            let mut writer = write::XzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        #[cfg(feature = "zstd")]
        Compression::Zstd => {
            let mut writer = write::ZstdEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        _ => unreachable!(),
    }
}

fn compute_crc(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}
