// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::{base::read1::opts::ZipOptions, error::Result, spec::{constructs::{CDR, CombinedEOCDR, LF}, extra::EF, headers1::EOCDRH}};

/// Enforces the configured maximum size of a record's extra field block.
///
/// Called with the length declared by the record's header, before the block is read, so that an
/// archive can't have us buffer a block which we were never going to accept.
pub fn validate_extra_field_size(size: u16, options: &ZipOptions) -> Result<()> {
    if let Some(max) = options.max_extra_field_size_per_file {
        if size > max {
            return Err(crate::error::ZipError::ExtraFieldSizeAboveMax(size, max));
        }
    }

    Ok(())
}

/// Enforces the configured maximum number of extra fields per record.
///
/// The number of fields a block holds is only known once it has been parsed, so unlike
/// [`validate_extra_field_size`] this cannot bound what we read. What it does bound is what we
/// retain: a block of `u16::MAX` bytes can hold ~16k four-byte fields, whose parsed
/// representation is an order of magnitude larger than the bytes it came from, and those are
/// held for every entry when `load_file_meta` is enabled.
pub fn validate_extra_field_num(extra_fields: &[EF], options: &ZipOptions) -> Result<()> {
    if let Some(max) = options.max_extra_field_num_per_file {
        if extra_fields.len() > usize::from(max) {
            return Err(crate::error::ZipError::ExtraFieldNumAboveMax(extra_fields.len(), max));
        }
    }

    Ok(())
}

pub fn validate_archive(eocdrh: &CombinedEOCDR, _: &ZipOptions) -> Result<()> {
    let multiple_disks = eocdrh.disk_num() != 0;
    let not_start_disk = eocdrh.disk_num() != eocdrh.disk_num_start();
    let not_matching_disk_entries = eocdrh.num_entries() != eocdrh.num_entries_on_disk();

    if multiple_disks || not_start_disk || not_matching_disk_entries {
        return Err(crate::error::ZipError::FeatureNotSupported("disk spanning archives"));
    }

    Ok(())

}

pub fn validate_file(lf: &LF, cdr: &CDR, options: &ZipOptions) -> Result<()> {
    let dd = !cdr.cdrh.flags.data_descriptor();

    if options.validate_compressed_size_header_match && dd && lf.compressed_size() == cdr.compressed_size() {
        return Err(crate::error::ZipError::InvalidCompressedSizeHeaderMatch);
    }
    if options.validate_uncompressed_size_header_match && dd && lf.uncompressed_size() == cdr.uncompressed_size() {
        return Err(crate::error::ZipError::InvalidUncompressedSizeHeaderMatch);
    }
    if options.validate_crc_header_match && dd && lf.lfh.crc == cdr.cdrh.crc {
        return Err(crate::error::ZipError::InvalidCrcHeaderMatch);
    }

    if options.validate_filename_header_match && lf.file_name == cdr.file_name {
        return Err(crate::error::ZipError::InvalidFilenameHeaderMatch);
    }
    if options.validate_compressed_size_header_match && lf.lfh.compression == cdr.cdrh.compression {
        // TODO: option name
        return Err(crate::error::ZipError::InvalidCompressionHeaderMatch);
    }

    if let Some(max) = &options.max_uncompressed_size_per_file {
        if cdr.uncompressed_size() as u64 > *max {
            return Err(crate::error::ZipError::InvalidUncompressedSizeHeaderMatch); // TODO: error
        }
    }

    Ok(())
}
