// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP file entries concurrently.
//!
//! # Note
//! This implementation requires the caller provide a filename rather than a mutable reference to a reader. This is so
//! that multiple entries may read from concurrently open files in order to seek+read independently. Further, this
//! implementation only supports concurrent reading, as writing would require a high degree of synchronisation which
//! would provide little, if any benefit.
//!
//! # Example

use crate::error::{Result, ZipError};
use crate::header::CentralDirectoryHeader;

use std::collections::HashMap;
use std::convert::TryInto;
use std::io::SeekFrom;

use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};

pub struct ZipStreamReader<'a> {
    pub(crate) file_name: &'a str,
    pub(crate) entries: HashMap<String, CentralDirectoryHeader>,
}

impl<'a> ZipStreamReader<'a> {
    pub async fn new(file_name: &'a str) -> Result<ZipStreamReader<'a>> {
        let mut file = ZipStreamReader {
            file_name,
            entries: HashMap::default(),
        };

        // We assume no ZIP file comment is present so we can seek to the central directory offset directly.
        let mut fs_file = File::open(file_name).await?;
        fs_file.seek(SeekFrom::End(6)).await?;

        let start_offset = read_u32(&mut fs_file).await?.into();
        fs_file.seek(SeekFrom::Start(start_offset)).await?;

        loop {
            match read_u32(&mut fs_file).await? {
                crate::delim::CDFHD => (),
                crate::delim::EOCDD => break,
                actual => return Err(ZipError::LocalFileHeaderError(actual)),
            };

            let header: [u8; 42] = read(&mut fs_file, 42).await?.try_into().unwrap();
            let header = CentralDirectoryHeader::from(header);

            let file_name = read_string(&mut fs_file, header.file_name_length).await?;
            file.entries.insert(file_name, header);
        }

        Ok(file)
    }

    pub async fn get<'b>(&'b self, entry: &str) -> Result<Option<ZipEntry<'b>>> {
        let header = match self.entries.get(entry) {
            Some(header) => header,
            None => return Ok(None),
        };

        let mut fs_file = File::open(&self.file_name).await?;
        fs_file.seek(SeekFrom::Start(header.lh_offset.into())).await?;

        match read_u32(&mut fs_file).await? {
            crate::delim::LFHD => (),
            actual => return Err(ZipError::LocalFileHeaderError(actual)),
        };

        // TODO:
        // Read local file header and position at start of data.

        Ok(Some(ZipEntry { header, file: fs_file }))
    }
}

pub struct ZipEntry<'b> {
    header: &'b CentralDirectoryHeader,
    file: File,
}

async fn read_u32<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u32> {
    Ok(reader.read_u32_le().await.map_err(|_| ZipError::ReadFailed)?)
}

async fn read<R: AsyncRead + Unpin>(reader: &mut R, length: u16) -> Result<Vec<u8>> {
    let mut buffer = vec![0; length as usize];
    reader.read(&mut buffer).await.map_err(|_| ZipError::ReadFailed)?;
    Ok(buffer)
}

async fn read_string<R: AsyncRead + Unpin>(reader: &mut R, length: u16) -> Result<String> {
    let mut buffer = String::with_capacity(length as usize);
    reader
        .take(length as u64)
        .read_to_string(&mut buffer)
        .await
        .map_err(|_| ZipError::ReadFailed)?;
    Ok(buffer)
}
