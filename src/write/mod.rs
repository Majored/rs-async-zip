// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports writing ZIP files (unimplemented).

use crate::error::Result;
use crate::header::{CentralDirectoryHeader, GeneralPurposeFlag, LocalFileHeader};
use crate::Compression;

use std::io::Cursor;

use async_compression::tokio::write::{BzEncoder, DeflateEncoder, LzmaEncoder, XzEncoder, ZstdEncoder};
use chrono::Utc;
use crc32fast::Hasher;
use tokio::io::{AsyncWrite, AsyncWriteExt};

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
    written: usize,
}

impl<'a, W: AsyncWrite + Unpin> ZipFileWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer, cd_entries: Vec::new(), written: 0 }
    }

    pub async fn write_entry(&mut self, opts: EntryOptions<'_>, raw_data: &[u8]) -> Result<()> {
        let mut _compressed_data: Option<Vec<u8>> = None;
        let compressed_data = match &opts.compression {
            Compression::Stored => raw_data,
            _ => {
                _compressed_data = Some(compress(&opts.compression, raw_data).await);
                _compressed_data.as_ref().unwrap()
            }
        };

        let (mod_time, mod_date) = crate::utils::chrono_to_zip_time(&Utc::now());

        let lf_header = LocalFileHeader {
            compressed_size: compressed_data.len() as u32,
            uncompressed_size: raw_data.len() as u32,
            compression: opts.compression.to_u16(),
            crc: compute_crc(raw_data),
            extra_field_length: 0,
            file_name_length: opts.filename.as_bytes().len() as u16,
            mod_time,
            mod_date,
            version: 0,
            flags: GeneralPurposeFlag { data_descriptor: false, encrypted: false },
        };

        let cd_header = CentralDirectoryHeader {
            v_made_by: 0,
            v_needed: 0,
            compressed_size: lf_header.compressed_size,
            uncompressed_size: lf_header.uncompressed_size,
            compression: lf_header.compression,
            crc: lf_header.crc,
            extra_field_length: lf_header.extra_field_length,
            file_name_length: lf_header.file_name_length,
            file_comment_length: 0,
            mod_time: lf_header.mod_time,
            mod_date: lf_header.mod_date,
            flags: lf_header.flags,
            disk_start: 0,
            inter_attr: 0,
            exter_attr: 0,
            lh_offset: self.written as u32,
        };

        self.written += self.writer.write(&crate::delim::LFHD.to_le_bytes()).await?;
        self.written += self.writer.write(&lf_header.to_slice()).await?;
        self.written += self.writer.write(opts.filename.as_bytes()).await?;
        self.written += self.writer.write(compressed_data).await?;

        self.cd_entries.push(cd_header);

        Ok(())
    }

    pub fn stream_write_entry(&mut self, opts: EntryOptions) -> Result<()> {
        unimplemented!();
    }

    pub fn close(self) -> Result<()> {
        unimplemented!();
    }
}

async fn compress(compression: &Compression, data: &[u8]) -> Vec<u8> {
    // TODO: Reduce reallocations of Vec by making a lower-bound estimate of the length reduction and
    // pre-initialising the Vec to that length. Then truncate() to the actual number of bytes written.
    match compression {
        Compression::Deflate => {
            let mut writer = DeflateEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Bz => {
            let mut writer = BzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Lzma => {
            let mut writer = LzmaEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Xz => {
            let mut writer = XzEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
            writer.into_inner().into_inner()
        }
        Compression::Zstd => {
            let mut writer = ZstdEncoder::new(Cursor::new(Vec::new()));
            writer.write_all(data).await.unwrap();
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
