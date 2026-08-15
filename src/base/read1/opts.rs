// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[derive(Clone)]
pub struct Options {
    /// Whether we load the full CDR file meta into memory on open.
    /// 
    /// Reduces memory usage, but requries seeking to the central directory for every file read.
    /// Can be significantly slower as read-ahead buffers (both user and OS level) are likely discarded.
    pub load_file_meta: bool,

    // TODO: validate CDR vs LFH: filename, crc32, compressed size, uncompressed size, etc.
    // TODO: validate against read file.

    /// The method to use for locating the EOCDR.
    /// 
    /// 0 = scan backwards from the end of the file, looking for the EOCDR signature.
    /// 1 = scan backwards from the end of the file, looking for the EOCDR signature.
    pub eocdr_locate_method: u8,

    /// Whether to validate that the compressed size in the local file header matches the central directory record.
    pub validate_compressed_size_header_match: bool,

    /// Whether to validate that the uncompressed size in the local file header matches the central directory record.
    pub validate_uncompressed_size_header_match: bool,

    /// Whether to validate that the CRC in the local file header matches the central directory record.
    pub validate_crc_header_match: bool,

    /// Whether to validate that the filename in the local file header matches the central directory record.
    pub validate_filename_header_match: bool,

    /// Whether to validate that the compression method in the local file header matches the central directory record.
    pub validate_compression_header_match: bool,

    // RODO: max num of files
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