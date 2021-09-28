// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use crate::header::EndOfCentralDirectoryHeader;
use crate::read::{ZipEntry, ZipEntryReader};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use std::io::SeekFrom;
use std::convert::TryInto;

pub struct ZipFileReader<'a, R: AsyncRead + AsyncSeek + Unpin> {
    pub(crate) reader: &'a mut R,
    pub(crate) entries: Vec<(u32, ZipEntry)>,
}

impl<'a, R: AsyncRead + AsyncSeek + Unpin> ZipFileReader<'a, R> {
    pub async fn new(reader: &'a mut R) -> Result<ZipFileReader<'a, R>> {
        let entries = read_cd(reader).await?;
        Ok(ZipFileReader { reader, entries })
    }
}

pub(crate) async fn read_cd<R: AsyncRead + AsyncSeek + Unpin>(reader: &mut R) -> Result<Vec<(u32, ZipEntry)>> {
    // Assume no ZIP comment exists for the moment so we can seek directly to EOCD header.
    reader.seek(SeekFrom::End(22)).await?;

    if read_u32(reader).await? != crate::delim::EOCDD {
        return Err(ZipError::FeatureNotSupported("ZIP file comment"));
    }

    let mut buffer: [u8; 18] = [0; 18];
    reader.read(&mut buffer).await?;
    let header = EndOfCentralDirectoryHeader::from(buffer);

    if header.disk_num != header.start_cent_dir_disk || header.num_of_entries != header.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    reader.seek(SeekFrom::Start(header.cent_dir_offset.into())).await?;
    let mut entries = Vec::with_capacity(header.num_of_entries.into());

    loop {
        todo!();
    }

    Ok(entries)
}

async fn read_u32<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u32> {
    Ok(reader.read_u32_le().await.map_err(|_| ZipError::ReadFailed)?)
}

async fn read_u16<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u16> {
    Ok(reader.read_u16_le().await.map_err(|_| ZipError::ReadFailed)?)
}