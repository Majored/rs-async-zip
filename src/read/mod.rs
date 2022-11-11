// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP files.

pub mod mem;
pub mod seek;

#[cfg(feature = "fs")]
pub mod fs;

pub(crate) mod io;

use crate::entry::{ZipEntry, ZipEntryMeta};
use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::spec::attribute::AttributeCompatibility;
use crate::spec::compression::Compression;
use crate::spec::consts::{LFH_LENGTH, SIGNATURE_LENGTH};
use crate::spec::header::{CentralDirectoryRecord, EndOfCentralDirectoryHeader};

use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, SeekFrom};

pub(crate) async fn file<R>(mut reader: R) -> Result<ZipFile>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    let eocdr_offset = crate::read::io::locator::eocdr(&mut reader).await?;

    reader.seek(SeekFrom::Start(eocdr_offset)).await?;
    let eocdr = EndOfCentralDirectoryHeader::from_reader(&mut reader).await?;
    let comment = crate::read::io::read_string(&mut reader, eocdr.file_comm_length.into()).await?;

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdr.disk_num != eocdr.start_cent_dir_disk || eocdr.num_of_entries != eocdr.num_of_entries_disk {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    reader.seek(SeekFrom::Start(eocdr.cent_dir_offset.into())).await?;
    let (entries, metas) = crate::read::cd(&mut reader, eocdr.num_of_entries.into()).await?;

    Ok(ZipFile { entries, metas, comment, zip64: false })
}

pub(crate) async fn cd<R>(mut reader: R, num_of_entries: u64) -> Result<(Vec<ZipEntry>, Vec<ZipEntryMeta>)>
where
    R: AsyncRead + Unpin,
{
    let num_of_entries: usize = num_of_entries.try_into().map_err(|_| ZipError::TargetZip64Unsupported)?;
    let mut entries = Vec::with_capacity(num_of_entries);
    let mut metas = Vec::with_capacity(num_of_entries);

    for _ in 0..num_of_entries {
        let (entry, meta) = cd_record(&mut reader).await?;

        entries.push(entry);
        metas.push(meta);
    }

    Ok((entries, metas))
}

pub(crate) async fn cd_record<R>(mut reader: R) -> Result<(ZipEntry, ZipEntryMeta)>
where
    R: AsyncRead + Unpin,
{
    let header = CentralDirectoryRecord::from_reader(&mut reader).await?;
    let filename = crate::read::io::read_string(&mut reader, header.file_name_length.into()).await?;
    let compression = Compression::try_from(header.compression)?;
    let extra_field = crate::read::io::read_bytes(&mut reader, header.extra_field_length.into()).await?;
    let comment = crate::read::io::read_string(reader, header.file_comment_length.into()).await?;
    #[cfg(feature = "date")]
    let last_modification_date = crate::spec::date::zip_date_to_chrono(header.mod_date, header.mod_time);

    let entry = ZipEntry {
        filename,
        compression,
        compression_level: async_compression::Level::Default,
        attribute_compatibility: AttributeCompatibility::Unix,
        /// FIXME: Default to Unix for the moment
        crc32: header.crc,
        uncompressed_size: header.uncompressed_size,
        compressed_size: header.compressed_size,
        #[cfg(feature = "date")]
        last_modification_date,
        internal_file_attribute: header.inter_attr,
        external_file_attribute: header.exter_attr,
        extra_field,
        comment,
    };

    let meta = ZipEntryMeta { general_purpose_flag: header.flags, file_offset: header.lh_offset as u64 };

    Ok((entry, meta))
}

pub(crate) fn compute_data_offset(entry: &ZipEntry, meta: &ZipEntryMeta) -> u64 {
    let header_length = SIGNATURE_LENGTH + LFH_LENGTH;
    let trailing_length = entry.comment().as_bytes().len() + entry.extra_field().len();

    meta.file_offset + (header_length as u64) + (trailing_length as u64)
}
