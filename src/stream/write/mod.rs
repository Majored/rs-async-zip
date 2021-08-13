// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports stream writing ZIP files.
//! 
//! # Note
//! Force Deflate for the time being.
//! 
//! # Example

use crate::error::{Result, ZipError};
use crate::header::{LocalFileHeader, GeneralPurposeFlag};

use std::convert::TryInto;

use crc32fast::Hasher;
use chrono::{DateTime, Utc, Datelike, Timelike};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use async_compression::tokio::write::DeflateEncoder;

/// A type accepted as output to ZipStreamWriter.
pub(crate) type AsyncWriter = dyn AsyncWrite + Unpin + Send;

pub struct ZipStreamWriterGuard<'a, 'b> {
    compressed_writer: DeflateEncoder<&'b mut &'a mut AsyncWriter>,
    crc_hasher: Hasher,
    compressed_size: u32,
    uncompressed_size: u32,
}

impl<'a, 'b> ZipStreamWriterGuard<'a, 'b> {
    pub fn from_writer(raw_writer: &'b mut ZipStreamWriter<'a>) -> Self {
        Self {
            compressed_writer: DeflateEncoder::new(&mut raw_writer.writer),
            crc_hasher: Hasher::new(),
            compressed_size: 0,
            uncompressed_size: 0,
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.uncompressed_size += data.len() as u32;

        match self.compressed_writer.write(data).await {
            Ok(written) => self.compressed_size += written as u32,
            Err(_) => return Err(ZipError::ReadFailed),
        };

        self.crc_hasher.update(data);

        Ok(())
    }

    pub async fn close(self) -> Result<()> {
        let inner_borrow = self.compressed_writer.into_inner();
        let mut data_descriptor: Vec<u8> = Vec::with_capacity(128);

        data_descriptor.append(&mut crate::delim::DDD.to_le_bytes().to_vec());
        data_descriptor.append(&mut self.crc_hasher.finalize().to_le_bytes().to_vec());
        data_descriptor.append(&mut self.compressed_size.to_le_bytes().to_vec());
        data_descriptor.append(&mut self.uncompressed_size.to_le_bytes().to_vec());

        match inner_borrow.write(&data_descriptor).await {
            Ok(_) => Ok(()),
            Err(_) => Err(ZipError::ReadFailed),
        }
    }
}

pub struct ZipStreamWriter<'a> {
    writer: &'a mut AsyncWriter,
    current_entry: Option<()>,
}

impl<'a> ZipStreamWriter<'a> {
    /// Constructs a new instance from a mutable reference to a writer.
    pub fn new(writer: &'a mut AsyncWriter) -> Self {
        Self { writer, current_entry: None }
    }

    /// Writes the local file header for a new entry and places the writer at the end of it, ready to start writing the
    /// actual file's data.
    /// 
    /// This function will return Err if we're currently already writing a file and haven't closed the entry.
    pub async fn new_entry<'b>(&'b mut self, file_name: &str) -> Result<ZipStreamWriterGuard<'a, 'b>> {
        if self.current_entry.is_some() {
            return Err(ZipError::ReadFailed);
        }

        let (mod_time, mod_date) = chrono_to_zip_time(&Utc::now());

        let header = LocalFileHeader {
            compressed_size: 0,
            uncompressed_size: 0,
            crc: 0,
            compression: 8,
            flags: GeneralPurposeFlag {
                data_descriptor: true,
                encrypted: false,
            },
            file_name_length: file_name.as_bytes().len() as u16,
            extra_field_length: 0,
            mod_time,
            mod_date,
            version: 0,
        }.to_slice();

        match self.writer.write(&crate::delim::LFHD.to_le_bytes()).await {
            Ok(_) => (),
            Err(_) => return Err(ZipError::ReadFailed),
        };

        match self.writer.write(&header).await {
            Ok(_) => (),
            Err(_) => return Err(ZipError::ReadFailed),
        };

        match self.writer.write(file_name.as_bytes()).await {
            Ok(_) => (),
            Err(_) => return Err(ZipError::ReadFailed),
        };

        Ok(ZipStreamWriterGuard::from_writer(self))
    }

    /// Writes out the current entry's data descriptor and modifies internal state to allow a new entry to be started.
    /// 
    /// This function will return Err if we're currently not writing any file.
    pub async fn close_entry(&mut self) -> Result<()> {
        if self.current_entry.is_none() {
            return Err(ZipError::ReadFailed);
        }

        

        Ok(())
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        if self.current_entry.is_none() {
            return Err(ZipError::ReadFailed);
        }

        match self.writer.write(data).await {
            Ok(_) => (),
            Err(_) => return Err(ZipError::ReadFailed),
        };

        Ok(())
    }
}

pub fn chrono_to_zip_time(dt: &DateTime<Utc>) -> (u16, u16) {
    let year: u16 = (((dt.date().year() - 1980) << 9) & 0xFE00).try_into().unwrap();
    let month: u16 = ((dt.date().month() << 5) & 0x1E0).try_into().unwrap();
    let day: u16 = (dt.date().day() & 0x1F).try_into().unwrap();

    let hour: u16 = ((dt.time().hour() << 11) & 0x1F).try_into().unwrap();
    let min: u16 = ((dt.time().minute() << 5) & 0x7E0).try_into().unwrap();
    let second: u16 = ((dt.time().second() >> 1) & 0x1F).try_into().unwrap();

    (hour | min | second, year | month | day)
}