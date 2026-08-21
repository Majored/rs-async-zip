// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::{base::read1::opts::ZipOptions, error::Result, spec::{constructs::{CDR, CEOCDR, LF}, extra::EF}};

/// Enforces the configured maximum size of a record's extra field block.
///
/// Called with the length declared by the record's header, before the block is read, so that an
/// archive can't have us buffer a block which we were never going to accept.
pub fn validate_extra_field_size(size: u16, options: &ZipOptions) -> Result<()> {
    if size > options.max_extra_field_size_per_file {
        return Err(crate::error::ZipError::ExtraFieldSizeAboveMax(options.max_extra_field_size_per_file));
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
    if extra_fields.len() > options.max_extra_field_num_per_file.into() {
        return Err(crate::error::ZipError::ExtraFieldNumAboveMax(options.max_extra_field_num_per_file));
    }

    Ok(())
}

pub fn validate_archive(ceocdr: &CEOCDR, opts: &ZipOptions) -> Result<()> {
    let multiple_disks = ceocdr.disk_num()? != 0;
    let not_start_disk = ceocdr.disk_num()? != ceocdr.disk_num_start()?;
    let not_matching_disk_entries = ceocdr.num_entries()? != ceocdr.num_entries_on_disk()?;

    if multiple_disks || not_start_disk || not_matching_disk_entries {
        return Err(crate::error::ZipError::FeatureNotSupported("disk spanning archives"));
    }

    if ceocdr.cd_size()? > opts.max_cd_size_in_bytes {
        return Err(crate::error::ZipError::CDSizeAboveMax(opts.max_cd_size_in_bytes));
    }
    if ceocdr.num_entries()? > opts.max_cd_num_files {
        return Err(crate::error::ZipError::NumFilesAboveMax(opts.max_cd_num_files));
    }

    Ok(())

}

pub fn validate_file(lf: &LF, cdr: &CDR, options: &ZipOptions) -> Result<()> {
    let dd = cdr.cdrh.gpf.data_descriptor();

    if options.validate_compressed_size_header_match && !dd && lf.compressed_size()? != cdr.compressed_size()? {
        return Err(crate::error::ZipError::CompressedSizeHeaderMismatch);
    }
    if options.validate_uncompressed_size_header_match && !dd && lf.uncompressed_size()? != cdr.uncompressed_size()? {
        return Err(crate::error::ZipError::UncompressedSizeHeaderMismatch);
    }
    if options.validate_crc_header_match && !dd && lf.lfh.crc != cdr.cdrh.crc {
        return Err(crate::error::ZipError::CrcHeaderMismatch);
    }

    if options.validate_file_name_header_match && lf.insecure_file_name != cdr.insecure_file_name {
        return Err(crate::error::ZipError::FileNameHeaderMismatch);
    }
    if options.validate_compression_header_match && lf.lfh.compression != cdr.cdrh.compression {
        return Err(crate::error::ZipError::CompressionHeaderMismatch);
    }
    if options.validate_gpf_header_match && lf.lfh.gpf != cdr.cdrh.gpf {
        return Err(crate::error::ZipError::GPFHeaderMismatch);
    }

    if cdr.uncompressed_size()? > options.max_uncompressed_size_per_file {
        return Err(crate::error::ZipError::UncompressedSizeAboveMax(options.max_uncompressed_size_per_file));
    }
    if cdr.compressed_size()? > options.max_compressed_size_per_file {
        return Err(crate::error::ZipError::CompressedSizeAboveMax(options.max_compressed_size_per_file));
    }

    Ok(())
}

pub fn validate_file_eof(lf: &LF, crc: u32, read: u64, opts: &ZipOptions) -> std::io::Result<()> {
    if opts.validate_crc_match_against_read && crc != lf.lfh.crc {
        return Err(std_invalid_data_err(crate::error::ZipError::CRC32CheckError));
    }
    if opts.validate_uncompressed_size_match_against_read {
        let declared = lf.uncompressed_size().map_err(std_invalid_data_err)?;

        if read != declared {
            return Err(std_invalid_data_err(crate::error::ZipError::UncompressedSizeReadMismatch(read, declared)));
        }
    }

    // TODO: validate compressed size, but we need to do a calculation of the source reader offsets.

    Ok(())
}

pub(crate) fn std_invalid_data_err(error: crate::error::ZipError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}
