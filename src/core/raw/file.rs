// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::SeekFrom;

use futures_lite::io::{AsyncBufRead, AsyncSeek, AsyncSeekExt};
use crate::{core::raw::{cdr::CentralDirectoryRecord, eocdr::EndOfCentralDirectoryRecord}, error::{Result, ZipError}};
use crate::core::raw::zip64::eocdl::Zip64EndOfCentralDirectoryLocator;

use super::zip64::eocdr::Zip64EndOfCentralDirectoryRecord;

#[derive(Clone, Debug)]
pub struct RawFile {
    pub eocdr: EndOfCentralDirectoryRecord,
    pub zip64_eocdl: Option<Zip64EndOfCentralDirectoryLocator>,
    pub zip64_eocdr: Option<Zip64EndOfCentralDirectoryRecord>,
    pub records: Vec<CentralDirectoryRecord>,
}

#[tracing::instrument(skip(reader))]
pub async fn read(mut reader: impl AsyncBufRead + AsyncSeek + Unpin, interval: u64) -> Result<RawFile> {
    let eocdr_offset = crate::core::raw::eocdr::locate(&mut reader, interval).await?;
    reader.seek(SeekFrom::Start(eocdr_offset)).await?;

    let eocdr = crate::core::raw::eocdr::read(&mut reader).await?;
    let zip64_eocdl: Option<Zip64EndOfCentralDirectoryLocator> = None;
    let zip64_eocdr: Option<Zip64EndOfCentralDirectoryRecord> = None;

    let records = crate::core::cd::read(&mut reader, eocdr.total_number_of_entries_in_the_central_directory).await?;
    
    Ok(RawFile { eocdr, zip64_eocdl, zip64_eocdr, records })
}
