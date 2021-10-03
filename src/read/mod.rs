// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod concurrent;
pub mod seek;
pub mod stream;

use crate::error::{Result, ZipError};
use crate::header::CentralDirectoryHeader;
use crate::Compression;

use std::pin::Pin;
use std::task::{Context, Poll};

use chrono::{DateTime, TimeZone, Utc};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

pub(crate) type Reader = dyn AsyncRead + Unpin;

///
#[derive(Clone)]
pub struct ZipEntry {
    pub(crate) name: String,
    pub(crate) comment: Option<String>,
    pub(crate) data_descriptor: bool,
    pub(crate) crc32: Option<u32>,
    pub(crate) uncompressed_size: Option<u32>,
    pub(crate) compressed_size: Option<u32>,
    pub(crate) last_modified: DateTime<Utc>,
    pub(crate) extra: Option<Vec<u8>>,
    pub(crate) compression: Compression,
}

impl ZipEntry {
    pub fn from_raw(header: CentralDirectoryHeader, filename: String) -> Result<Self> {
        Ok(ZipEntry {
            name: filename,
            comment: None,
            data_descriptor: header.flags.data_descriptor,
            crc32: Some(header.crc),
            uncompressed_size: Some(header.uncompressed_size),
            compressed_size: Some(header.compressed_size),
            last_modified: zip_date_to_chrono(header.mod_date, header.mod_time),
            extra: None,
            compression: Compression::from_u16(header.compression)?,
        })
    }

    /// Returns a shared reference to the entry's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an optional shared reference to the entry's comment.
    pub fn comment(&self) -> Option<&str> {
        match &self.comment {
            Some(comment) => Some(comment),
            None => None,
        }
    }

    /// Returns whether or not a data descriptor exists for the entry (ie. whether or not it was stream written).
    pub fn data_descriptor(&self) -> bool {
        self.data_descriptor
    }

    /// Returns an optional CRC32 value for the entry.
    pub fn crc32(&self) -> Option<u32> {
        self.crc32
    }

    pub fn compressed_size(&self) -> Option<u32> {
        self.compressed_size
    }

    pub fn uncompressed_size(&self) -> Option<u32> {
        self.uncompressed_size
    }

    pub fn last_modified(&self) -> &DateTime<Utc> {
        &self.last_modified
    }

    pub fn extra(&self) -> Option<&Vec<u8>> {
        self.extra.as_ref()
    }

    pub fn compression(&self) -> &Compression {
        &self.compression
    }
}

/// A ZIP entry reader over some generic reader which could implement decompression.
///
/// # Note
/// This type will never implmement AsyncSeek, even if the underlying implementation from this crate implies seek
/// capabilities.
pub struct ZipEntryReader {
    pub(crate) entry: ZipEntry,
    pub(crate) reader: Box<Reader>,
}

impl ZipEntryReader {
    pub fn entry(&self) -> &ZipEntry {
        &self.entry
    }
}

impl AsyncRead for ZipEntryReader {
    fn poll_read(mut self: Pin<&mut Self>, c: &mut Context<'_>, b: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(c, b)
    }
}

pub(crate) fn zip_date_to_chrono(date: u16, time: u16) -> DateTime<Utc> {
    let years = (((date & 0xFE00) >> 9) + 1980).into();
    let months = ((date & 0x1E0) >> 5).into();
    let days = (date & 0x1F).into();

    let hours = ((time & 0x1F) >> 11).into();
    let mins = ((time & 0x7E0) >> 5).into();
    let secs = ((time & 0x1F) << 1).into();

    Utc.ymd(years, months, days).and_hms(hours, mins, secs)
}

pub(crate) async fn read_u32<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u32> {
    Ok(reader.read_u32_le().await.map_err(|_| ZipError::ReadFailed)?)
}

pub(crate) async fn read_string<R: AsyncRead + Unpin>(reader: &mut R, length: u16) -> Result<String> {
    let mut buffer = String::with_capacity(length as usize);
    reader
        .take(length as u64)
        .read_to_string(&mut buffer)
        .await
        .map_err(|_| ZipError::ReadFailed)?;
    Ok(buffer)
}
