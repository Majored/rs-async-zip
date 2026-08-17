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

    /// Validates the CRC of the actual read data matches the expected value.
    /// Users must call [`ZipFileReader::validate()`] after reading all data to perform this check.
    pub validate_crc_match_against_read: bool,

    /// Validates the uncompressed size of the actual read data matches the expected value.
    /// Users must call [`ZipFileReader::validate()`] after reading all data to perform this check.
    /// 
    pub validate_uncompressed_size_match_against_read: bool,

    /// Enforces a maximum uncompressed size per file. Attempts to open the file for reading will
    /// fail, or if the file declares a smaller uncompressed size than is actually the case,
    /// reading the file will fail if the limit is exceeded.
    pub max_uncompressed_size_per_file: Option<u64>,

    /// Enforces a maximum size, in bytes, of the extra field block of each file.
    ///
    /// Applied to the length declared by the local file header or central directory record
    /// before the block itself is read, so an archive cannot force us to buffer a block we
    /// were never going to accept. Note that the format caps this at [`u16::MAX`] anyway.
    pub max_extra_field_size_per_file: Option<u16>,

    /// Enforces a maximum number of extra fields per file.
    ///
    /// Unlike [`Self::max_extra_field_size_per_file`], this can only be applied once the block
    /// has been parsed, as the number of fields it holds isn't declared anywhere. Use it to
    /// bound the fields retained per entry (see [`Self::load_file_meta`]) rather than to bound
    /// what we read; the size limit above is the one which does that.
    pub max_extra_field_num_per_file: Option<u16>,

    // TODO: max cd dize
    // TODO: max compressed size per file
    // TODO: max uncompressed size per file
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
            validate_crc_match_against_read: true,
            max_uncompressed_size_per_file: None,
            validate_uncompressed_size_match_against_read: true,
            max_extra_field_size_per_file: None,
            max_extra_field_num_per_file: None,
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

// Generates a builder setter that assigns `param` to the same-named field on `self.options`.
macro_rules! builder_setter {
    ($name:ident, $param:ident: $ty:ty) => {
        pub fn $name(mut self, $param: $ty) -> Self {
            self.options.$name = $param;
            self
        }
    };
}

impl ZipOptionsBuilder {
    pub fn new() -> Self {
        Self { options: ZipOptions::default() }
    }

    builder_setter!(load_file_meta, load: bool);
    builder_setter!(eocdr_locate_method, method: u8);
    builder_setter!(validate_compressed_size_header_match, validate: bool);
    builder_setter!(validate_uncompressed_size_header_match, validate: bool);
    builder_setter!(validate_crc_header_match, validate: bool);
    builder_setter!(validate_filename_header_match, validate: bool);
    builder_setter!(validate_compression_header_match, validate: bool);
    builder_setter!(validate_num_central_directory_files, validate: bool);
    builder_setter!(max_num_central_directory_files, max: Option<u64>);
    builder_setter!(validate_crc_match_against_read, validate: bool);
    builder_setter!(validate_uncompressed_size_match_against_read, validate: bool);
    builder_setter!(max_uncompressed_size_per_file, max: Option<u64>);
    builder_setter!(max_extra_field_size_per_file, max: Option<u16>);
    builder_setter!(max_extra_field_num_per_file, max: Option<u16>);
}
