// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[derive(Clone, Debug)]
/// A set of options for configuring the behavior of a ZIP archive reader.
pub struct ZipOptions {
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
    pub validate_num_central_directory_files: bool,

    pub max_num_central_directory_files: Option<u64>,

    // TODO: max cd dize
    // TODO: max ef size per file
    // TODO: max ef num per file
}

impl Default for ZipOptions {
    fn default() -> Self {
        Self {
            load_file_meta: true,
            eocdr_locate_method: 0,
            validate_compressed_size_header_match: true,
            validate_uncompressed_size_header_match: true,
            validate_crc_header_match: true,
            validate_filename_header_match: true,
            validate_compression_header_match: true,
            validate_num_central_directory_files: true,
            max_num_central_directory_files: None,
        }
    }
}

impl From<ZipOptionsBuilder> for ZipOptions {
    fn from(builder: ZipOptionsBuilder) -> Self {
        builder.options
    }
}

/// A builder for [`ZipOptions`] which define the behavior of a ZIP archive reader.
pub struct ZipOptionsBuilder {
    options: ZipOptions,
}

impl ZipOptionsBuilder {
    pub fn new() -> Self {
        Self { options: ZipOptions::default() }
    }

    pub fn load_file_meta(mut self, load: bool) -> Self {
        self.options.load_file_meta = load;
        self
    }

    pub fn eocdr_locate_method(mut self, method: u8) -> Self {
        self.options.eocdr_locate_method = method;
        self
    }

    pub fn validate_compressed_size_header_match(mut self, validate: bool) -> Self {
        self.options.validate_compressed_size_header_match = validate;
        self
    }

    pub fn validate_uncompressed_size_header_match(mut self, validate: bool) -> Self {
        self.options.validate_uncompressed_size_header_match = validate;
        self
    }

    pub fn validate_crc_header_match(mut self, validate: bool) -> Self {
        self.options.validate_crc_header_match = validate;
        self
    }

    pub fn validate_filename_header_match(mut self, validate: bool) -> Self {
        self.options.validate_filename_header_match = validate;
        self
    }

    pub fn validate_compression_header_match(mut self, validate: bool) -> Self {
        self.options.validate_compression_header_match = validate;
        self
    }

    pub fn validate_num_central_directory_files(mut self, validate: bool) -> Self {
        self.options.validate_num_central_directory_files = validate;
        self
    }

    pub fn max_num_central_directory_files(mut self, max: Option<u64>) -> Self {
        self.options.max_num_central_directory_files = max;
        self
    }
}
