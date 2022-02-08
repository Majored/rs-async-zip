// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#437
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

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#444
#[derive(Copy, Clone)]
pub struct GeneralPurposeFlag {
    pub encrypted: bool,
    pub data_descriptor: bool,
}

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#4312
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

// https://github.com/Majored/rs-async-zip/blob/main/src/spec/APPNOTE.md#4316
pub struct EndOfCentralDirectoryHeader {
    pub(crate) disk_num: u16,
    pub(crate) start_cent_dir_disk: u16,
    pub(crate) num_of_entries_disk: u16,
    pub(crate) num_of_entries: u16,
    pub(crate) size_cent_dir: u32,
    pub(crate) cent_dir_offset: u32,
    pub(crate) file_comm_length: u16,
}
