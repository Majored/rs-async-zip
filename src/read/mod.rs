// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP files.

pub mod mem;
pub mod seek;

#[cfg(feature = "fs")]
pub mod fs;

pub(crate) mod io;

use crate::entry::{StoredZipEntry, ZipEntry};
use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::spec::date::ZipDateTime;
use crate::spec::attribute::AttributeCompatibility;
use crate::spec::compression::Compression;
use crate::spec::consts::CDH_SIGNATURE;
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
    let entries = crate::read::cd(&mut reader, eocdr.num_of_entries.into()).await?;

    Ok(ZipFile { entries, comment, zip64: false })
}

pub(crate) async fn cd<R>(mut reader: R, num_of_entries: u64) -> Result<Vec<StoredZipEntry>>
where
    R: AsyncRead + Unpin,
{
    let num_of_entries = num_of_entries.try_into().map_err(|_| ZipError::TargetZip64NotSupported)?;
    let mut entries = Vec::with_capacity(num_of_entries);

    for _ in 0..num_of_entries {
        let entry = cd_record(&mut reader).await?;
        entries.push(entry);
    }

    Ok(entries)
}

pub(crate) async fn cd_record<R>(mut reader: R) -> Result<StoredZipEntry>
where
    R: AsyncRead + Unpin,
{
    crate::utils::assert_signature(&mut reader, CDH_SIGNATURE).await?;

    let header = CentralDirectoryRecord::from_reader(&mut reader).await?;
    let filename = crate::read::io::read_string(&mut reader, header.file_name_length.into()).await?;
    let compression = Compression::try_from(header.compression)?;
    let extra_field = crate::read::io::read_bytes(&mut reader, header.extra_field_length.into()).await?;
    let comment = crate::read::io::read_string(reader, header.file_comment_length.into()).await?;

    let entry = ZipEntry {
        filename,
        compression,
        #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
        compression_level: async_compression::Level::Default,
        attribute_compatibility: AttributeCompatibility::Unix,
        /// FIXME: Default to Unix for the moment
        crc32: header.crc,
        uncompressed_size: header.uncompressed_size,
        compressed_size: header.compressed_size,
        last_modification_date: ZipDateTime {date: header.mod_date, time: header.mod_time},
        internal_file_attribute: header.inter_attr,
        external_file_attribute: header.exter_attr,
        extra_field,
        comment,
    };

    // general_purpose_flag: header.flags,
    Ok(StoredZipEntry { entry, file_offset: header.lh_offset as u64 })
}
