// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::{base::read1::opts::ZipOptions, error::Result, spec::constructs::{CDR, LF}};

pub fn validate(lf: &LF, cdr: &CDR, options: &ZipOptions) -> Result<()> {
    // TODO: we're still performing these checks even if disabled. Reorder.

    let compressed_match = lf.lfh.compressed_size == cdr.cdrh.compressed_size;
    let uncompressed_match = lf.lfh.uncompressed_size == cdr.cdrh.uncompressed_size;
    let crc_match = lf.lfh.crc == cdr.cdrh.crc;
    let filename_match = lf.file_name == cdr.file_name;
    let compression_match = lf.lfh.compression == cdr.cdrh.compression;

    // These checks can only be performed with actually set LFH values.
    if !cdr.cdrh.flags.data_descriptor() {
        if options.validate_compressed_size_header_match && !compressed_match {
            return Err(crate::error::ZipError::InvalidCompressedSizeHeaderMatch);
        }
        if options.validate_uncompressed_size_header_match && !uncompressed_match {
            return Err(crate::error::ZipError::InvalidUncompressedSizeHeaderMatch);
        }
        if options.validate_crc_header_match && !crc_match {
            return Err(crate::error::ZipError::InvalidCrcHeaderMatch);
        }
    }

    if options.validate_filename_header_match && !filename_match {
        return Err(crate::error::ZipError::InvalidFilenameHeaderMatch);
    }
    if options.validate_compressed_size_header_match && !compression_match {
        return Err(crate::error::ZipError::InvalidCompressionHeaderMatch);
    }

    Ok(())
}
