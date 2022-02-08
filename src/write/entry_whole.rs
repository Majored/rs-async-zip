// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::Result;
use crate::spec::compression::Compression;
use crate::spec::header::{CentralDirectoryHeader, GeneralPurposeFlag, LocalFileHeader};
use crate::write::{CentralDirectoryEntry, EntryOptions, ZipFileWriter};

use std::io::Cursor;

use async_compression::tokio::write::{BzEncoder, DeflateEncoder, LzmaEncoder, XzEncoder, ZstdEncoder};
use chrono::Utc;
use crc32fast::Hasher;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct EntryWholeWriter<'a, 'b, 'c, W: AsyncWrite + Unpin> {
    writer: &'b mut ZipFileWriter<'a, W>,
    opts: EntryOptions,
    data: &'c [u8],
}

impl<'a, 'b, 'c, W: AsyncWrite + Unpin> EntryWholeWriter<'a, 'b, 'c, W> {
    pub fn from_raw(writer: &'b mut ZipFileWriter<'a, W>, opts: EntryOptions, data: &'c [u8]) -> Self {
        Self { writer, opts, data }
    }

    pub async fn write(self) -> Result<()> {
        let mut _compressed_data: Option<Vec<u8>> = None;
        let compressed_data = match &self.opts.compression {
            Compression::Stored => self.data,
            _ => {
                _compressed_data = Some(compress(&self.opts.compression, self.data).await);
                _compressed_data.as_ref().unwrap()
            }
        };

        let (mod_time, mod_date) = crate::spec::date::chrono_to_zip_time(&Utc::now());

        let lf_header = LocalFileHeader {
            compressed_size: compressed_data.len() as u32,
            uncompressed_size: self.data.len() as u32,
            compression: self.opts.compression.to_u16(),
            crc: compute_crc(self.data),
            extra_field_length: self.opts.extra.len() as u16,
            file_name_length: self.opts.filename.as_bytes().len() as u16,
            mod_time,
            mod_date,
            version: 0,
            flags: GeneralPurposeFlag { data_descriptor: false, encrypted: false },
        };

        let header = CentralDirectoryHeader {
            v_made_by: 0,
            v_needed: 0,
            compressed_size: lf_header.compressed_size,
            uncompressed_size: lf_header.uncompressed_size,
            compression: lf_header.compression,
            crc: lf_header.crc,
            extra_field_length: lf_header.extra_field_length,
            file_name_length: lf_header.file_name_length,
            file_comment_length: self.opts.comment.len() as u16,
            mod_time: lf_header.mod_time,
            mod_date: lf_header.mod_date,
            flags: lf_header.flags,
            disk_start: 0,
            inter_attr: 0,
            exter_attr: 0,
            lh_offset: self.writer.writer.offset() as u32,
        };

        self.writer.writer.write_all(&crate::spec::signature::LOCAL_FILE_HEADER.to_le_bytes()).await?;
        self.writer.writer.write_all(&lf_header.to_slice()).await?;
        self.writer.writer.write_all(self.opts.filename.as_bytes()).await?;
        self.writer.writer.write_all(&self.opts.extra).await?;
        self.writer.writer.write_all(compressed_data).await?;

        self.writer.cd_entries.push(CentralDirectoryEntry { header, opts: self.opts });

        Ok(())
    }
}

async fn compress(compression: &Compression, data: &[u8]) -> Vec<u8> {
    // TODO: Reduce reallocations of Vec by making a lower-bound estimate of the length reduction and
    // pre-initialising the Vec to that length. Then truncate() to the actual number of bytes written.
    match compression {
        Compression::Deflate => {
            let mut writer = DeflateEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Bz => {
            let mut writer = BzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Lzma => {
            let mut writer = LzmaEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Xz => {
            let mut writer = XzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.shutdown().await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Zstd => {
            let mut writer = ZstdEncoder::new(Cursor::new(Vec::new()));
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
