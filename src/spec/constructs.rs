// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::headers1::{CDRH, EFH, EOCDL64H, EOCDR64H, EOCDRH, HeaderSize, LFH};
use binrw::{binrw, BinRead, BinWrite};

// Constructing blocks from raw headers & bytes sequences (like filenames, comments, etc).

// Local file
pub struct LF {
    pub lfh: LFH,
    pub file_name: Vec<u8>,
    pub extra_fields: Vec<EF>,
}

// Central directory record
#[derive(Clone, Debug)]
pub struct CDR {
    pub cdrh: CDRH,
    pub file_name: Vec<u8>,
    pub extra_fields: Vec<EF>,
    pub file_comment: Vec<u8>,
}

// End of central directory record
#[derive(Debug)]
pub struct EOCDR {
    pub eocdrh: EOCDRH,
    pub file_comment: Vec<u8>,
}

// Extra field
#[derive(Clone, Debug)]
pub struct EF {
    pub efh: EFH,
    pub data: EFData,
}

#[derive(Clone, Debug)]
pub enum EFData {
    Zip64ExtendedInformation(Zip64ExtendedInformation),
    UnicodeFilename(Vec<u8>),
    UnicodeComment(Vec<u8>),
    Unknown(Vec<u8>),
}

// A ZIP64 combined end of central directory record
pub struct CombinedEOCDR {
    pub eocdr: EOCDR,
    pub eocdr64: Option<EOCDR64H>,
    pub eocdl64: Option<EOCDL64H>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// ZIP64 end of central directory locator header
pub struct Zip64ExtendedInformation {
    uncompressed_size: u64,
    compressed_size: u64,
    relative_offset: u64,
    disk_number_start: u32,
}

impl HeaderSize for Zip64ExtendedInformation {
    const SIZE: usize = 0;
}