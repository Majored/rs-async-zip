// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::write::{ZipFileWriter, EntryOptions};
use crate::error::Result;
use crate::header::{CentralDirectoryHeader, GeneralPurposeFlag, LocalFileHeader};
use crate::write::compressed_writer::CompressedAsyncWriter;
use crate::write::offset_writer::OffsetAsyncWriter;
use crate::write::CentralDirectoryEntry;

use std::io::Error;
use std::pin::Pin;
use std::task::{Poll, Context};

use chrono::Utc;
use crc32fast::Hasher;
use tokio::io::{AsyncWrite, AsyncWriteExt};

// Taking a mutable reference ensures that no two writers can act upon the same ZipFileWriter. 
pub struct EntryStreamWriter<'a, 'b, W: AsyncWrite + Unpin> {
    writer: OffsetAsyncWriter<CompressedAsyncWriter<'b, &'a mut W>>,
    cd_entries: &'b mut Vec<CentralDirectoryEntry>,
    options: EntryOptions,
    hasher: Hasher,
    lfh: LocalFileHeader,
    lfh_offset: usize,
    data_offset: usize,
}

impl<'a, 'b, W: AsyncWrite + Unpin> EntryStreamWriter<'a, 'b, W> {
    pub async fn from_raw(writer: &'b mut ZipFileWriter<'a, W>, options: EntryOptions) -> Result<EntryStreamWriter<'a, 'b, W>> {
        let lfh_offset = writer.writer.offset();
        let lfh = EntryStreamWriter::write_lfh(writer, &options).await?;
        let data_offset = writer.writer.offset();

        let cd_entries = &mut writer.cd_entries;
        let writer = OffsetAsyncWriter::from_raw(CompressedAsyncWriter::from_raw(&mut writer.writer, options.compression));

        Ok(EntryStreamWriter {
            writer,
            cd_entries,
            options,
            lfh,
            lfh_offset,
            data_offset,
            hasher: Hasher::new(),
        })
    }

    async fn write_lfh(writer: &'b mut ZipFileWriter<'a, W>, options: &EntryOptions) -> Result<LocalFileHeader> {
        let (mod_time, mod_date) = crate::utils::chrono_to_zip_time(&Utc::now());
    
        let lfh = LocalFileHeader {
            compressed_size: 0,
            uncompressed_size: 0,
            compression: options.compression.to_u16(),
            crc: 0,
            extra_field_length: options.extra.len() as u16,
            file_name_length: options.filename.as_bytes().len() as u16,
            mod_time,
            mod_date,
            version: 0,
            flags: GeneralPurposeFlag { data_descriptor: true, encrypted: false },
        };

        writer.writer.write_all(&crate::delim::LFHD.to_le_bytes()).await?;
        writer.writer.write_all(&lfh.to_slice()).await?;
        writer.writer.write_all(options.filename.as_bytes()).await?;
        writer.writer.write_all(&options.extra).await?;

        Ok(lfh)
    }

    pub async fn close(self) {
        let crc = self.hasher.finalize();
        let uncompressed_size = self.writer.offset() as u32;
        let inner_writer = self.writer.into_inner().into_inner();
        let compressed_size = (inner_writer.offset() - self.data_offset) as u32;

        let cdh = CentralDirectoryHeader {
            compressed_size,
            uncompressed_size,
            crc,
            v_made_by: 0,
            v_needed: 0,
            compression: self.lfh.compression,
            extra_field_length: self.lfh.extra_field_length,
            file_name_length: self.lfh.file_name_length,
            file_comment_length: self.options.comment.len() as u16,
            mod_time: self.lfh.mod_time,
            mod_date: self.lfh.mod_date,
            flags: self.lfh.flags,
            disk_start: 0,
            inter_attr: 0,
            exter_attr: 0,
            lh_offset: self.lfh_offset as u32,
        };

        self.cd_entries.push(CentralDirectoryEntry { header: cdh, opts: self.options });
    }
}

impl<'a, 'b, W: AsyncWrite + Unpin> AsyncWrite for EntryStreamWriter<'a, 'b, W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        self.hasher.update(buf);
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}
