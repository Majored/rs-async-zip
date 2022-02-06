// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// local file header signature     4 bytes  (0x04034b50)
// version needed to extract       2 bytes
// general purpose bit flag        2 bytes
// compression method              2 bytes
// last mod file time              2 bytes
// last mod file date              2 bytes
// compressed size                 4 bytes
// uncompressed size               4 bytes
// file name length                2 bytes
// extra field length              2 bytes
//
// file name (variable size)
// extra field (variable size)
pub struct LocalFileHeader {
    pub version: u16,
    pub flags: GeneralPurposeFlag,
    pub compression: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
}

// Bit 0: If set, indicates that the file is encrypted.
//
// (For Method 6 - Imploding)
// Bit 1: If the compression method used was type 6,
//        Imploding, then this bit, if set, indicates
//        an 8K sliding dictionary was used.  If clear,
//        then a 4K sliding dictionary was used.
//
// Bit 2: If the compression method used was type 6,
//        Imploding, then this bit, if set, indicates
//        3 Shannon-Fano trees were used to encode the
//        sliding dictionary output.  If clear, then 2
//        Shannon-Fano trees were used.
//
// (For Methods 8 and 9 - Deflating)
// Bit 2  Bit 1
//   0      0    Normal (-en) compression option was used.
//   0      1    Maximum (-exx/-ex) compression option was used.
//   1      0    Fast (-ef) compression option was used.
//   1      1    Super Fast (-es) compression option was used.
//
// (For Method 14 - LZMA)
// Bit 1: If the compression method used was type 14,
//        LZMA, then this bit, if set, indicates
//        an end-of-stream (EOS) marker is used to
//        mark the end of the compressed data stream.
//        If clear, then an EOS marker is not present
//        and the compressed data size must be known
//        to extract.
//
// Note:  Bits 1 and 2 are undefined if the compression
//        method is any other.
//
// Bit 3: If this bit is set, the fields crc-32, compressed
//        size and uncompressed size are set to zero in the
//        local header.  The correct values are put in the
//        data descriptor immediately following the compressed
//        data.  (Note: PKZIP version 2.04g for DOS only
//        recognizes this bit for method 8 compression, newer
//        versions of PKZIP recognize this bit for any
//        compression method.)
//
// Bit 4: Reserved for use with method 8, for enhanced
//        deflating.
//
// Bit 5: If this bit is set, this indicates that the file is
//        compressed patched data.  (Note: Requires PKZIP
//        version 2.70 or greater)
//
// Bit 6: Strong encryption.  If this bit is set, you MUST
//        set the version needed to extract value to at least
//        50 and you MUST also set bit 0.  If AES encryption
//        is used, the version needed to extract value MUST
//        be at least 51. See the section describing the Strong
//        Encryption Specification for details.  Refer to the
//        section in this document entitled "Incorporating PKWARE
//        Proprietary Technology into Your Product" for more
//        information.
//
// Bit 7: Currently unused.
//
// Bit 8: Currently unused.
//
// Bit 9: Currently unused.
//
// Bit 10: Currently unused.
//
// Bit 11: Language encoding flag (EFS).  If this bit is set,
//         the filename and comment fields for this file
//         MUST be encoded using UTF-8. (see APPENDIX D)
//
// Bit 12: Reserved by PKWARE for enhanced compression.
//
// Bit 13: Set when encrypting the Central Directory to indicate
//         selected data values in the Local Header are masked to
//         hide their actual values.  See the section describing
//         the Strong Encryption Specification for details.  Refer
//         to the section in this document entitled "Incorporating
//         PKWARE Proprietary Technology into Your Product" for
//         more information.
//
// Bit 14: Reserved by PKWARE for alternate streams.
//
// Bit 15: Reserved by PKWARE.
#[derive(Copy, Clone)]
pub struct GeneralPurposeFlag {
    pub encrypted: bool,
    pub data_descriptor: bool,
}

// central file header signature   4 bytes  (0x02014b50)
// version made by                 2 bytes
// version needed to extract       2 bytes
// general purpose bit flag        2 bytes
// compression method              2 bytes
// last mod file time              2 bytes
// last mod file date              2 bytes
// crc-32                          4 bytes
// compressed size                 4 bytes
// uncompressed size               4 bytes
// file name length                2 bytes
// extra field length              2 bytes
// file comment length             2 bytes
// disk number start               2 bytes
// internal file attributes        2 bytes
// external file attributes        4 bytes
// relative offset of local header 4 bytes
//
// file name (variable size)
// extra field (variable size)
// file comment (variable size)
pub struct CentralDirectoryHeader {
    pub v_made_by: u16,
    pub v_needed: u16,
    pub flags: GeneralPurposeFlag,
    pub compression: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
    pub file_comment_length: u16,
    pub disk_start: u16,
    pub inter_attr: u16,
    pub exter_attr: u32,
    pub lh_offset: u32,
}

// end of central dir signature    4 bytes  (0x06054b50)
// number of this disk             2 bytes
// number of the disk with the
// start of the central directory  2 bytes
// total number of entries in the
// central directory on this disk  2 bytes
// total number of entries in
// the central directory           2 bytes
// size of the central directory   4 bytes
// offset of start of central
// directory with respect to
// the starting disk number        4 bytes
// .ZIP file comment length        2 bytes
// .ZIP file comment       (variable size)
pub struct EndOfCentralDirectoryHeader {
    pub(crate) disk_num: u16,
    pub(crate) start_cent_dir_disk: u16,
    pub(crate) num_of_entries_disk: u16,
    pub(crate) num_of_entries: u16,
    pub(crate) size_cent_dir: u32,
    pub(crate) cent_dir_offset: u32,
    pub(crate) file_comm_length: u16,
}
