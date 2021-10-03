// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};
use crate::header::{CentralDirectoryHeader, EndOfCentralDirectoryHeader};
use crate::read::ZipEntry;

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt};

use std::io::SeekFrom;

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

    if super::read_u32(reader).await? != crate::delim::EOCDD {
        return Err(ZipError::FeatureNotSupported("ZIP file comment"));
    }

    let eocdh = EndOfCentralDirectoryHeader::from_reader(reader).await?;

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdh.disk_num != eocdh.start_cent_dir_disk || eocdh.num_of_entries != eocdh.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    reader.seek(SeekFrom::Start(eocdh.cent_dir_offset.into())).await?;
    let mut entries = Vec::with_capacity(eocdh.num_of_entries.into());

    for _ in 0..eocdh.num_of_entries {
        if super::read_u32(reader).await? != crate::delim::CDFHD {
            return Err(ZipError::ReadFailed); // Alter error message.
        }

        // Ignore file extra & comment for the moment.
        let header = CentralDirectoryHeader::from_reader(reader).await?;
        let offset = header.lh_offset;
        let filename = super::read_string(reader, header.file_name_length).await?;

        entries.push((offset, ZipEntry::from_raw(header, filename)?));
    }

    Ok(entries)
}
