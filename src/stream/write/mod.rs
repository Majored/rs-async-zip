// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports stream writing ZIP files.
//! 
//! # Note
//! Force Deflate for the time being.
//! 
//! # Example

use crate::error::{Result};
use crate::header::{LocalFileHeader, GeneralPurposeFlag};
use crate::Compression;
use crate::opts::ZipEntryOptions;

use std::convert::TryInto;

use crc32fast::Hasher;
use chrono::{DateTime, Utc, Datelike, Timelike};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use async_compression::tokio::write::{DeflateEncoder, BzEncoder, LzmaEncoder, ZstdEncoder, XzEncoder};

use std::pin::Pin;
use std::task::{Poll, Context};

/// An enum of possible concrete compression decoders which are supported by this crate.
pub(crate) enum CompressionWriter<'a, 'b> {
    Stored(WriteAdapter<'b, 'a>),
    Deflate(DeflateEncoder<WriteAdapter<'b, 'a>>),
    Bz(BzEncoder<WriteAdapter<'b, 'a>>),
    Lzma(LzmaEncoder<WriteAdapter<'b, 'a>>),
    Zstd(ZstdEncoder<WriteAdapter<'b, 'a>>),
    Xz(XzEncoder<WriteAdapter<'b, 'a>>),
}

impl<'a, 'b> CompressionWriter<'a, 'b> {
    pub fn to_writer(adapter: WriteAdapter<'b, 'a>, compression: Compression) -> Self {
        match compression {
            Compression::Stored => CompressionWriter::Stored(adapter),
            Compression::Deflate => CompressionWriter::Deflate(DeflateEncoder::new(adapter)),
            Compression::Bz => CompressionWriter::Bz(BzEncoder::new(adapter)),
            Compression::Lzma => CompressionWriter::Lzma(LzmaEncoder::new(adapter)),
            Compression::Zstd => CompressionWriter::Zstd(ZstdEncoder::new(adapter)),
            Compression::Xz => CompressionWriter::Xz(XzEncoder::new(adapter)),
        }
    }
}

/// A type accepted as output to ZipStreamWriter.
pub(crate) type AsyncWriter = dyn AsyncWrite + Unpin + Send;

pub struct WriteAdapter<'b, 'a> {
    inner: &'b mut &'a mut AsyncWriter,
    written: u32,
}

impl<'b, 'a> AsyncWrite for WriteAdapter<'b, 'a> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<std::result::Result<usize, tokio::io::Error>> {
        self.written += buf.len() as u32;
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), tokio::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<std::result::Result<(), tokio::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

pub struct CentralDirectoryEntry {
    file_name: String,
    v_made_by: u16,
    v_needed: u16,
    flags: GeneralPurposeFlag,
    compression: u16,
    mod_time: u16,
    mod_date: u16,
    crc: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    file_name_length: u16,
    extra_field_length: u16,
    file_comment_length: u16,
    disk_start: u16,
    inter_attr: u16,
    exter_attr: u32,
    lh_offset: u32,
}

impl CentralDirectoryEntry {
    pub fn to_slice(&self) -> [u8; 42] {
        let mut data: Vec<u8> = Vec::with_capacity(42);

        data.append(&mut self.v_made_by.to_ne_bytes().to_vec());
        data.append(&mut self.v_needed.to_ne_bytes().to_vec());
        data.append(&mut self.flags.to_slice().to_vec());
        data.append(&mut self.compression.to_ne_bytes().to_vec());
        data.append(&mut self.mod_time.to_ne_bytes().to_vec());
        data.append(&mut self.mod_date.to_ne_bytes().to_vec());
        data.append(&mut self.crc.to_ne_bytes().to_vec());
        data.append(&mut self.compressed_size.to_ne_bytes().to_vec());
        data.append(&mut self.uncompressed_size.to_ne_bytes().to_vec());
        data.append(&mut self.file_name_length.to_ne_bytes().to_vec());
        data.append(&mut self.extra_field_length.to_ne_bytes().to_vec());
        data.append(&mut self.file_comment_length.to_ne_bytes().to_vec());
        data.append(&mut self.disk_start.to_ne_bytes().to_vec());
        data.append(&mut self.inter_attr.to_ne_bytes().to_vec());
        data.append(&mut self.exter_attr.to_ne_bytes().to_vec());
        data.append(&mut self.lh_offset.to_ne_bytes().to_vec());

        data.try_into().unwrap()
    }
}

#[derive(Default)]
pub struct WriterInnerState {
    written: usize,
    entries: Vec<CentralDirectoryEntry>,
}

pub struct ZipStreamWriterGuard<'a, 'b> {
    compressed_writer: CompressionWriter<'a, 'b>,
    state: &'b mut WriterInnerState,
    header: LocalFileHeader,
    file_name: String,
    start_offset: usize,
    crc_hasher: Hasher,
    uncompressed_size: u32,
}

impl<'b, 'a> AsyncWrite for ZipStreamWriterGuard<'a, 'b> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<std::result::Result<usize, tokio::io::Error>> {
        self.uncompressed_size += buf.len() as u32;
        self.crc_hasher.update(buf);

        match &mut self.compressed_writer {
            CompressionWriter::Stored(inner) => Pin::new(inner).poll_write(cx, buf),
            CompressionWriter::Deflate(inner) => Pin::new(inner).poll_write(cx, buf),
            CompressionWriter::Bz(inner) => Pin::new(inner).poll_write(cx, buf),
            CompressionWriter::Lzma(inner) => Pin::new(inner).poll_write(cx, buf),
            CompressionWriter::Zstd(inner) => Pin::new(inner).poll_write(cx, buf),
            CompressionWriter::Xz(inner) => Pin::new(inner).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), tokio::io::Error>> {
        match &mut self.compressed_writer {
            CompressionWriter::Stored(inner) => Pin::new(inner).poll_flush(cx),
            CompressionWriter::Deflate(inner) => Pin::new(inner).poll_flush(cx),
            CompressionWriter::Bz(inner) => Pin::new(inner).poll_flush(cx),
            CompressionWriter::Lzma(inner) => Pin::new(inner).poll_flush(cx),
            CompressionWriter::Zstd(inner) => Pin::new(inner).poll_flush(cx),
            CompressionWriter::Xz(inner) => Pin::new(inner).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<std::result::Result<(), tokio::io::Error>> {
        match &mut self.compressed_writer {
            CompressionWriter::Stored(inner) => Pin::new(inner).poll_shutdown(cx),
            CompressionWriter::Deflate(inner) => Pin::new(inner).poll_shutdown(cx),
            CompressionWriter::Bz(inner) => Pin::new(inner).poll_shutdown(cx),
            CompressionWriter::Lzma(inner) => Pin::new(inner).poll_shutdown(cx),
            CompressionWriter::Zstd(inner) => Pin::new(inner).poll_shutdown(cx),
            CompressionWriter::Xz(inner) => Pin::new(inner).poll_shutdown(cx),
        }
    }
}

impl<'a, 'b> ZipStreamWriterGuard<'a, 'b> {
    pub async fn from_writer(raw_writer: &'b mut ZipStreamWriter<'a>, entry_opts: ZipEntryOptions) -> Result<ZipStreamWriterGuard<'a, 'b>> {
        let start_offset = raw_writer.state.written;
        // Add check to see if we're already writing.

        let (mod_time, mod_date) = chrono_to_zip_time(&Utc::now());

        let header = LocalFileHeader {
            compressed_size: 0,
            uncompressed_size: 0,
            crc: 0,
            compression: entry_opts.compression.to_u16(),
            flags: GeneralPurposeFlag {
                data_descriptor: true,
                encrypted: false,
            },
            file_name_length: entry_opts.filename.as_bytes().len() as u16,
            extra_field_length: 0,
            mod_time,
            mod_date,
            version: 0,
        };

        raw_writer.writer.write(&crate::delim::LFHD.to_le_bytes()).await?;
        raw_writer.writer.write(&header.to_slice()).await?;
        raw_writer.writer.write(entry_opts.filename.as_bytes()).await?;

        raw_writer.state.written += 30 + entry_opts.filename.as_bytes().len();
        
        Ok(Self {
            compressed_writer: CompressionWriter::to_writer(
                WriteAdapter {inner: &mut raw_writer.writer, written: 0 }, entry_opts.compression
            ),
            state: &mut raw_writer.state,
            header,
            start_offset,
            file_name: entry_opts.filename,
            crc_hasher: Hasher::new(),
            uncompressed_size: 0,
        })
    }

    pub async fn close(mut self) -> Result<()> {
        match &mut self.compressed_writer {
            CompressionWriter::Deflate(inner) => inner.shutdown().await.unwrap(),
            CompressionWriter::Stored(inner) => inner.flush().await.unwrap(),
            _ => panic!("Unsupported")
        };

        let inner_borrow = match self.compressed_writer {
            CompressionWriter::Deflate(inner) => inner.into_inner(),
            CompressionWriter::Stored(inner) => inner,
            _ => panic!("Unsupported")
        };
        
        let mut data_descriptor: Vec<u8> = Vec::with_capacity(128);
        let crc = self.crc_hasher.finalize();

        data_descriptor.append(&mut crate::delim::DDD.to_le_bytes().to_vec());
        data_descriptor.append(&mut crc.to_le_bytes().to_vec());
        data_descriptor.append(&mut inner_borrow.written.to_le_bytes().to_vec());
        data_descriptor.append(&mut self.uncompressed_size.to_le_bytes().to_vec());

        let data_descriptior_size = inner_borrow.inner.write(&data_descriptor).await?;

        let central_entry = CentralDirectoryEntry {
            file_name: self.file_name,
            v_made_by: 0,
            v_needed: 0,
            flags: self.header.flags,
            compression: self.header.compression,
            mod_time: self.header.mod_time,
            mod_date: self.header.mod_date,
            crc,
            compressed_size: inner_borrow.written,
            uncompressed_size: self.uncompressed_size,
            file_name_length: self.header.file_name_length,
            extra_field_length: self.header.extra_field_length,
            file_comment_length: 0,
            disk_start: 0,
            inter_attr: 0,
            exter_attr: 0,
            lh_offset: self.start_offset as u32,
        };

        self.state.entries.push(central_entry);
        self.state.written += inner_borrow.written as usize + data_descriptior_size;

        Ok(())
    }
}

pub struct ZipStreamWriter<'a> {
    writer: &'a mut AsyncWriter,
    state: WriterInnerState,
}

impl<'a> ZipStreamWriter<'a> {
    /// Constructs a new instance from a mutable reference to a writer.
    pub fn new(writer: &'a mut AsyncWriter) -> Self {
        Self { writer, state: WriterInnerState::default() }
    }

    /// Writes the local file header for a new entry and places the writer at the end of it, ready to start writing the
    /// actual file's data.
    /// 
    /// This function will return Err if we're currently already writing a file and haven't closed the entry.
    pub async fn new_entry<'b>(&'b mut self, entry_opts: ZipEntryOptions) -> Result<ZipStreamWriterGuard<'a, 'b>> {
        ZipStreamWriterGuard::from_writer(self, entry_opts).await
    }

    pub async fn close(self) -> Result<()>{
        let cd_offset = self.state.written;
        let mut cd_size: u32 = 0;

        for entry in &self.state.entries {
            self.writer.write(&crate::delim::CDFHD.to_le_bytes()).await?;
            self.writer.write(&entry.to_slice()).await?;
            self.writer.write(&entry.file_name.as_bytes()).await?;
            
            cd_size += 4 + 42 + entry.file_name.as_bytes().len() as u32;
        }

        let header = EndOfCentralDirectoryHeader {
            disk_num: 0,
            start_cent_dir_disk: 0,
            num_of_entries_disk: self.state.entries.len() as u16,
            num_of_entries: self.state.entries.len() as u16,
            size_cent_dir: cd_size,
            cent_dir_offset: cd_offset as u32,
            file_comm_length: 0,
        };

        self.writer.write(&crate::delim::EOCDD.to_le_bytes()).await?;
        self.writer.write(&header.to_slice()).await?;

        Ok(())
    }
}

pub struct EndOfCentralDirectoryHeader {
    disk_num: u16,
    start_cent_dir_disk: u16,
    num_of_entries_disk: u16,
    num_of_entries: u16,
    size_cent_dir: u32,
    cent_dir_offset: u32,
    file_comm_length: u32,
}

impl EndOfCentralDirectoryHeader {
    pub fn to_slice(&self) -> [u8; 20] {
        let mut data: Vec<u8> = Vec::with_capacity(42);

        data.append(&mut self.disk_num.to_ne_bytes().to_vec());
        data.append(&mut self.start_cent_dir_disk.to_ne_bytes().to_vec());
        data.append(&mut self.num_of_entries_disk.to_ne_bytes().to_vec());
        data.append(&mut self.num_of_entries.to_ne_bytes().to_vec());
        data.append(&mut self.size_cent_dir.to_ne_bytes().to_vec());
        data.append(&mut self.cent_dir_offset.to_ne_bytes().to_vec());
        data.append(&mut self.file_comm_length.to_ne_bytes().to_vec());

        data.try_into().unwrap()
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