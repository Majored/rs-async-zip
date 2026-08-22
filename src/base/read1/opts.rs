// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[derive(Clone, Debug)]
/// A set of options for configuring the behavior of a ZIP archive reader.
/// 
/// # Usage
/// ```
/// # use async_zip::base::read1::ZipOptions;
/// // Use the struct update syntax to build options.
/// // Uses the default values but caps the allowed number of files to 16.
/// let options = ZipOptions { max_cd_num_files: 16, ..Default::default() };
/// 
/// // Use this to get a set of options suitable for reading untrusted ZIP archives.
/// // You must still handle file name security issues yourself.
/// let opts = ZipOptions { max_cd_num_files: 16, ..ZipOptions::untrusted() };
/// ```
/// 
/// # Breaking changes
/// We do **NOT** consider a field being added to this struct to be a breaking change.
/// This is because it would only be such if you were to define the struct without
/// using the struct update syntax, which we discourage (even if you intend on
/// defining a custom value for all current fields). The struct update syntax allows
/// you to pick up new fields no different than if we provided a separate builder.
pub struct ZipOptions {
    // OPTIMIZATION

    /// The method to use for locating the EOCDR. See [`ZipLocateMethod`].
    pub eocdr_locate_method: ZipLocateMethod,

    // VALIDATION

    /// Whether to validate that the compressed size in the LFH matches the CDR.
    pub validate_compressed_size_header_match: bool,

    /// Whether to validate that the uncompressed size in the LFH matches the CDR.
    pub validate_uncompressed_size_header_match: bool,

    /// Whether to validate that the CRC in the LFH matches the CDR.
    pub validate_crc_header_match: bool,

    /// Whether to validate that the file name in the LFH matches the CDR.
    pub validate_file_name_header_match: bool,

    /// Whether to validate that the compression method in the LFH matches the CDR.
    pub validate_compression_header_match: bool,

    /// Validates that the number of files in the CD matches the number declared in the CEOCDR.
    pub validate_num_cd_files: bool,

    /// Validates the CRC of the actual read data matches the expected value.
    /// [`Self::validate_file_on_eof`] must be enabled for this to be enforced.
    pub validate_crc_match_against_read: bool,

    /// Validates the uncompressed size of the actual read data matches the expected value.
    /// [`Self::validate_file_on_eof`] must be enabled for this to be enforced.
    pub validate_uncompressed_size_match_against_read: bool,

    /// Whether to validate the file when EOF is hit.
    pub validate_file_on_eof: bool,

    // Validates that the start of the reader is also the start of archive.
    // TODO
    // pub validate_sor_is_soa: bool,

    /// Validates that the end of the reader is also the end of archive.
    pub validate_eor_is_eoa: bool,

    /// Whether to validate that the GPF in the LFH matches the CDR.
    pub validate_gpf_header_match: bool,

    // LIMITS

    /// Enforces a maximum uncompressed size per file. Attempts to open the file for reading will
    /// fail, or if the file declares a smaller uncompressed size than is actually the case,
    /// reading the file will fail if the limit is exceeded.
    pub max_uncompressed_size_per_file: u64,

    /// Enforces a maximum compressed size per file. Attempts to open the file for reading will
    /// fail. Beyond that, the compressed size is enforced at the reader level, so there is no
    /// risk of reading more than the limit.
    pub max_compressed_size_per_file: u64,

    /// Enforces a maximum size, in bytes, of the extra field block of each file.
    ///
    /// Applied to the length declared by the local file header or central directory record
    /// before the block itself is read.
    pub max_extra_field_size_per_file: u16,

    /// Enforces a maximum number of extra fields per file.
    ///
    /// This can only be applied once the block has been parsed, as the number of fields it
    /// holds isn't declared anywhere.
    pub max_extra_field_num_per_file: u16,

    /// The maximum number of files to load from the CD into memory.
    /// 
    /// Lower values reduces memory usage, but require seeking to the CD for every file read.
    /// Can be significantly slower as read-ahead buffers (both user and OS level) are likely discarded.
    pub max_cd_num_files_load: u64,

    /// The maximum number of files in the CD.
    pub max_cd_num_files: u64,

    /// The maximum size of the central directory in bytes.
    pub max_cd_size_in_bytes: u64,
}

impl Default for ZipOptions {
    fn default() -> Self {
        Self {
            eocdr_locate_method: ZipLocateMethod::SeekBackReadLinearly,
            validate_compressed_size_header_match: true,
            validate_uncompressed_size_header_match: true,
            validate_crc_header_match: true,
            validate_file_name_header_match: true,
            validate_compression_header_match: true,
            validate_num_cd_files: true,
            validate_crc_match_against_read: true,
            validate_uncompressed_size_match_against_read: true,
            validate_file_on_eof: true,
            validate_gpf_header_match: true,
            // validate_sor_is_soa: true,
            validate_eor_is_eoa: true,
            max_uncompressed_size_per_file: u64::MAX,
            max_compressed_size_per_file: u64::MAX,
            max_cd_num_files: u64::MAX,
            max_cd_size_in_bytes: u64::MAX,
            max_cd_num_files_load: u64::MAX,
            max_extra_field_size_per_file: u16::MAX,
            max_extra_field_num_per_file: u16::MAX,
        }
    }
}

impl ZipOptions {
    /// Returns a set of options suitable for reading untrusted ZIP archives.
    /// 
    /// Every validation is enabled, and every limit is set to a value which comfortably fits
    /// archives produced by ordinary tooling. They are deliberately conservative, so please
    /// create an issue if you feel a limit should be reasonably raised.
    /// 
    /// Note that you _MUST_ still handle file name security issues yourself.
    pub fn untrusted() -> Self {
        Self {
            // 1 GiB. Bounds what a decompression bomb can expand to.
            max_uncompressed_size_per_file: 1024 * 1024 * 1024,

            // 1 GiB. Bounds how much of the underlying reader a single entry can consume.
            max_compressed_size_per_file: 1024 * 1024 * 1024,

            // 4 KiB. The extra fields written in practice (ZIP64, timestamps, UIDs/GIDs, NTFS)
            // total well under a few hundred bytes, so this leaves plenty of headroom.
            max_extra_field_size_per_file: 4 * 1024,

            // Typical archives carry a handful of fields per entry; 16 is well clear of that.
            max_extra_field_num_per_file: 16,

            // 64Ki entries, which is the limit of a non-ZIP64 archive.
            max_cd_num_files: 64 * 1024,

            // Load the same number as the maximum number of files set.
            max_cd_num_files_load: 64 * 1024,

            // 16 MiB, which is ~256 bytes per entry at the entry limit above.
            max_cd_size_in_bytes: 16 * 1024 * 1024,

            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A set of methods to use for locating the EOCDR in a ZIP archive.
pub enum ZipLocateMethod {
    /// The default. Optimized for buffered readers and OS-level file read-ahead.
    /// 
    /// Specifically, we seek to the first possible place the EOCDR could be and
    /// read forward linearly, finding all candidates. This maximizes buffer use
    /// by not seeking continuously (buffered readers will often discard their buffer
    /// on a seek), and allows the OS to make any read-ahead optimizations it can.
    SeekBackReadLinearly,
}
