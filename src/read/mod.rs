// Copyright (c) 2022-2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP files.

pub mod mem;
pub mod seek;
pub mod stream;

#[cfg(feature = "fs")]
pub mod fs;

pub(crate) mod io;

pub use crate::read::io::entry::ZipEntryReader;

use crate::entry::{StoredZipEntry, ZipEntry};
use crate::error::{Result, ZipError};
use crate::file::ZipFile;
use crate::spec::attribute::AttributeCompatibility;
use crate::spec::compression::Compression;
use crate::spec::consts::{CDH_SIGNATURE, LFH_SIGNATURE, NON_ZIP64_MAX_SIZE, ZIP64_EOCDL_LENGTH};
use crate::spec::date::ZipDateTime;
use crate::spec::header::{
    CentralDirectoryRecord, EndOfCentralDirectoryHeader, ExtraField, LocalFileHeader,
    Zip64EndOfCentralDirectoryLocator, Zip64EndOfCentralDirectoryRecord, Zip64ExtendedInformationExtraField,
};

use crate::read::io::combined_record::CombinedCentralDirectoryRecord;
use crate::spec::parse::parse_extra_fields;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, BufReader, SeekFrom};

/// The max buffer size used when parsing the central directory, equal to 20MiB.
const MAX_CD_BUFFER_SIZE: usize = 20 * 1024 * 1024;

pub(crate) async fn file<R>(mut reader: R) -> Result<ZipFile>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    // First find and parse the EOCDR.
    log::debug!("Locate EOCDR");
    let eocdr_offset = crate::read::io::locator::eocdr(&mut reader).await?;

    reader.seek(SeekFrom::Start(eocdr_offset)).await?;
    let eocdr = EndOfCentralDirectoryHeader::from_reader(&mut reader).await?;
    log::debug!("EOCDR: {:?}", eocdr);

    let comment = crate::read::io::read_string(&mut reader, eocdr.file_comm_length.into()).await?;

    // Check the 20 bytes before the EOCDR for the Zip64 EOCDL, plus an extra 4 bytes because the offset
    // does not include the signature. If the ECODL exists we are dealing with a Zip64 file.
    reader.seek(SeekFrom::Start(eocdr_offset - ZIP64_EOCDL_LENGTH - 4)).await?;
    let zip64_locator = Zip64EndOfCentralDirectoryLocator::try_from_reader(&mut reader).await?;
    log::debug!("Zip64EOCDL: {zip64_locator:?}");
    let zip64 = zip64_locator.is_some();
    let zip64_eocdr = if let Some(locator) = zip64_locator {
        reader.seek(SeekFrom::Start(locator.relative_offset + 4)).await?;
        Some(Zip64EndOfCentralDirectoryRecord::from_reader(&mut reader).await?)
    } else {
        None
    };
    log::debug!("Zip64EOCDR: {zip64_eocdr:?}");

    // Combine the two EOCDRs.
    let eocdr = CombinedCentralDirectoryRecord::combine(eocdr, zip64_eocdr);
    log::debug!("Combined directory: {eocdr:?}");

    // Outdated feature so unlikely to ever make it into this crate.
    if eocdr.disk_number != eocdr.disk_number_start_of_cd
        || eocdr.num_entries_in_directory != eocdr.num_entries_in_directory_on_disk
    {
        return Err(ZipError::FeatureNotSupported("Spanned/split files"));
    }

    // Find and parse the central directory.
    log::debug!("Read central directory");
    reader.seek(SeekFrom::Start(eocdr.offset_of_start_of_directory)).await?;

    // To avoid lots of small reads to `reader` when parsing the central directory, we use a BufReader that can read the whole central directory at once.
    // Because `eocdr.offset_of_start_of_directory` is a u64, we use MAX_CD_BUFFER_SIZE to prevent very large buffer sizes.
    let buf =
        BufReader::with_capacity(std::cmp::min(eocdr.offset_of_start_of_directory as _, MAX_CD_BUFFER_SIZE), reader);
    let entries = crate::read::cd(buf, eocdr.num_entries_in_directory, zip64).await?;

    Ok(ZipFile { entries, comment, zip64 })
}

pub(crate) async fn cd<R>(mut reader: R, num_of_entries: u64, zip64: bool) -> Result<Vec<StoredZipEntry>>
where
    R: AsyncRead + Unpin,
{
    let num_of_entries = num_of_entries.try_into().map_err(|_| ZipError::TargetZip64NotSupported)?;
    let mut entries = Vec::with_capacity(num_of_entries);

    for _ in 0..num_of_entries {
        let entry = cd_record(&mut reader, zip64).await?;
        entries.push(entry);
    }

    Ok(entries)
}

fn get_zip64_extra_field(extra_fields: &[ExtraField]) -> Option<&Zip64ExtendedInformationExtraField> {
    for field in extra_fields {
        if let ExtraField::Zip64ExtendedInformationExtraField(zip64field) = field {
            return Some(zip64field);
        }
    }
    None
}

fn get_combined_sizes(
    uncompressed_size: u32,
    compressed_size: u32,
    extra_field: &Option<&Zip64ExtendedInformationExtraField>,
) -> (u64, u64) {
    let mut uncompressed_size = uncompressed_size as u64;
    let mut compressed_size = compressed_size as u64;

    if let Some(extra_field) = extra_field {
        if uncompressed_size == NON_ZIP64_MAX_SIZE as u64 {
            uncompressed_size = extra_field.uncompressed_size;
        }
        if compressed_size == NON_ZIP64_MAX_SIZE as u64 {
            compressed_size = extra_field.compressed_size;
        }
    }

    (uncompressed_size, compressed_size)
}

pub(crate) async fn cd_record<R>(mut reader: R, _zip64: bool) -> Result<StoredZipEntry>
where
    R: AsyncRead + Unpin,
{
    crate::utils::assert_signature(&mut reader, CDH_SIGNATURE).await?;

    let header = CentralDirectoryRecord::from_reader(&mut reader).await?;
    let filename = crate::read::io::read_string(&mut reader, header.file_name_length.into()).await?;
    let compression = Compression::try_from(header.compression)?;
    let extra_field = crate::read::io::read_bytes(&mut reader, header.extra_field_length.into()).await?;
    let extra_fields = parse_extra_fields(extra_field)?;
    let comment = crate::read::io::read_string(reader, header.file_comment_length.into()).await?;

    let zip64_extra_field = get_zip64_extra_field(&extra_fields);
    let (uncompressed_size, compressed_size) =
        get_combined_sizes(header.uncompressed_size, header.compressed_size, &zip64_extra_field);

    let mut file_offset = header.lh_offset as u64;
    if let Some(zip64_extra_field) = zip64_extra_field {
        if file_offset == NON_ZIP64_MAX_SIZE as u64 {
            if let Some(offset) = zip64_extra_field.relative_header_offset {
                file_offset = offset;
            }
        }
    }

    let entry = ZipEntry {
        filename,
        compression,
        #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
        compression_level: async_compression::Level::Default,
        attribute_compatibility: AttributeCompatibility::Unix,
        /// FIXME: Default to Unix for the moment
        crc32: header.crc,
        uncompressed_size,
        compressed_size,
        last_modification_date: ZipDateTime { date: header.mod_date, time: header.mod_time },
        internal_file_attribute: header.inter_attr,
        external_file_attribute: header.exter_attr,
        extra_fields,
        comment,
    };

    log::debug!("Entry: {entry:?}, offset {file_offset}");

    // general_purpose_flag: header.flags,
    Ok(StoredZipEntry { entry, file_offset })
}

pub(crate) async fn lfh<R>(mut reader: R) -> Result<Option<ZipEntry>>
where
    R: AsyncRead + Unpin,
{
    match reader.read_u32_le().await? {
        actual if actual == LFH_SIGNATURE => (),
        actual if actual == CDH_SIGNATURE => return Ok(None),
        actual => return Err(ZipError::UnexpectedHeaderError(actual, LFH_SIGNATURE)),
    };

    let header = LocalFileHeader::from_reader(&mut reader).await?;
    let filename = crate::read::io::read_string(&mut reader, header.file_name_length.into()).await?;
    let compression = Compression::try_from(header.compression)?;
    let extra_field = crate::read::io::read_bytes(&mut reader, header.extra_field_length.into()).await?;
    let extra_fields = parse_extra_fields(extra_field)?;

    let zip64_extra_field = get_zip64_extra_field(&extra_fields);
    let (uncompressed_size, compressed_size) =
        get_combined_sizes(header.uncompressed_size, header.compressed_size, &zip64_extra_field);

    if header.flags.data_descriptor {
        return Err(ZipError::FeatureNotSupported(
            "stream reading entries with data descriptors (planned to be reintroduced)",
        ));
    }
    if header.flags.encrypted {
        return Err(ZipError::FeatureNotSupported("encryption"));
    }

    let entry = ZipEntry {
        filename,
        compression,
        #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
        compression_level: async_compression::Level::Default,
        attribute_compatibility: AttributeCompatibility::Unix,
        /// FIXME: Default to Unix for the moment
        crc32: header.crc,
        uncompressed_size,
        compressed_size,
        last_modification_date: ZipDateTime { date: header.mod_date, time: header.mod_time },
        internal_file_attribute: 0,
        external_file_attribute: 0,
        extra_fields,
        comment: String::new(),
    };

    Ok(Some(entry))
}
