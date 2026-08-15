// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[derive(Clone)]
pub struct Options {
    // Whether we load the full CDR file meta into memory on open.
    pub load_file_meta: bool,

    // TODO: validate CDR vs LFH: filename, crc32, compressed size, uncompressed size, etc.
    // TODO: validate against read file.

    // The method to use for locating the EOCDR. See module-level documentation for more information.
    pub eocdr_locate_method: u8,

    pub validate_compressed_size_header_match: bool,
    pub validate_uncompressed_size_header_match: bool,
    pub validate_crc_header_match: bool,
    pub validate_filename_header_match: bool,
    pub validate_compression_header_match: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            load_file_meta: true,
            eocdr_locate_method: 0,
            validate_compressed_size_header_match: true,
            validate_uncompressed_size_header_match: true,
            validate_crc_header_match: true,
            validate_filename_header_match: true,
            validate_compression_header_match: true,
        }
    }
}