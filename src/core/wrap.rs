// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::core::raw::cdr::CentralDirectoryRecord;
use crate::core::raw::eocdr::RawEndOfCentralDirectoryRecord;
use crate::core::raw::lfh::LocalFileHeader;

pub struct File {
    pub eocdr: RawEndOfCentralDirectoryRecord,
    pub records: Vec<CentralDirectoryRecord>,
    pub file_comment: Vec<u8>,
}

pub struct Entry {
    pub lfh: LocalFileHeader,
    pub extra_field: Vec<u8>,
}
