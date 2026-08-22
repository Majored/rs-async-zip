// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of higher-level constructs wrapping the primitive headers & variable-length data.

use binrw::binrw;

use crate::{error::{Result, ZipError}, spec::{extra::{EF, EFD, EFHID, EI64, efs}, headers1::{CDRH, EOCDL64H, EOCDR64H, EOCDRH, LFH}, string::ZipString}};

// Constructing blocks from raw headers & bytes sequences (like file names, comments, etc).
//
// Every construct here is parsed by binrw from an in-memory buffer. The lengths of the
// variable-length parts are always stated by the fixed-size header which precedes them,
// so the async caller only needs to know how many bytes to pull before handing the whole
// record over (see `headers1::read_record`).

#[binrw]
#[brw(little)]
#[derive(Debug)]
/// A local file. This struct provides ZIP64-aware accessors.
pub struct LF {
    pub lfh: LFH,
    #[br(count = lfh.file_name_length, args { utf8: lfh.gpf.language_encoding_flag() })]
    /// This file name is insecure. Untrusted values may contain path traversal sequences or similar.
    pub insecure_file_name: ZipString,
    #[br(parse_with = efs, args(lfh.extra_field_length.into()))]
    pub efs: Vec<EF>,
}

impl LF {
    // TODO: Add a Info-ZIP-aware file name accessor (also in CDR).

    /// A ZIP-64-aware accessor for the uncompressed size of the file.
    pub fn uncompressed_size(&self) -> Result<u64> {
        combined_accessor(self.lfh.uncompressed_size, &self.efs, |ei_data| ei_data.uncompressed_size)
    }

    /// A ZIP-64-aware accessor for the compressed size of the file.
    pub fn compressed_size(&self) -> Result<u64> {
        combined_accessor(self.lfh.compressed_size, &self.efs, |ei_data| ei_data.compressed_size)
    }
}

fn combined_accessor(zip32: u32, extra_fields: &[EF], accessor: impl Fn(&EI64) -> u64) -> Result<u64> {
    if zip32 != u32::MAX {
        return Ok(zip32.into());
    }

    let zip64ei = extra_fields.iter().find(|field| {
        matches!(field.efh.efid, EFHID::EI64)
    });
    if let Some(EF { efh: _, efd: EFD::EI64(data) }) = zip64ei {
        return Ok(accessor(data));
    }
    
    return Err(ZipError::NoZip64ExtendedInformation);
}

fn combined_accessor_ecodr_u16(zip16: u16, ceocdr: &CEOCDR, accessor: impl Fn(&EOCDR64H) -> u64) -> Result<u64> {
    if zip16 != u16::MAX {
        return Ok(zip16.into());
    }

    if let Some(record) = &ceocdr.eocdr64 {
        return Ok(accessor(record));
    }
    
    return Err(ZipError::NoZip64EOCDR);
}

fn combined_accessor_ecodr_u32(zip32: u32, ceocdr: &CEOCDR, accessor: impl Fn(&EOCDR64H) -> u64) -> Result<u64> {
    if zip32 != u32::MAX {
        return Ok(zip32.into());
    }

    if let Some(record) = &ceocdr.eocdr64 {
        return Ok(accessor(record));
    }
    
    return Err(ZipError::NoZip64EOCDR);
}

fn combined_accessor_ecodr_disk(zip32: u16, ceocdr: &CEOCDR, accessor: impl Fn(&EOCDR64H) -> u32) -> Result<u32> {
    if zip32 != u16::MAX {
        return Ok(zip32.into());
    }

    if let Some(record) = &ceocdr.eocdr64 {
        return Ok(accessor(record));
    }
    
    return Err(ZipError::NoZip64EOCDR);
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// A central directory record. This struct provides ZIP64-aware accessors.
pub struct CDR {
    /// The central directory record header.
    pub cdrh: CDRH,
    #[br(count = cdrh.file_name_length, args { utf8: cdrh.gpf.language_encoding_flag() })]
    /// This file name is insecure. Untrusted values may contain path traversal sequences or similar.
    pub insecure_file_name: ZipString,
    #[br(parse_with = efs, args(cdrh.extra_field_length.into()))]
    /// A list of extra fields.
    pub efs: Vec<EF>,
    #[br(count = cdrh.file_comment_length, args { utf8: cdrh.gpf.language_encoding_flag() })]
    pub file_comment: ZipString,
}

impl CDR {
    pub fn lfh_offset(&self) -> Result<u64> {
        // TODO: unwrap should be an err instead, in case it's a malformed ZIP.
        combined_accessor(self.cdrh.lh_offset, &self.efs, |ei_data| ei_data.relative_offset.unwrap())
    }

    /// A ZIP-64-aware accessor for the uncompressed size of the file.
    pub fn uncompressed_size(&self) -> Result<u64> {
        combined_accessor(self.cdrh.uncompressed_size, &self.efs, |ei_data| ei_data.uncompressed_size)
    }

    /// A ZIP-64-aware accessor for the compressed size of the file.
    pub fn compressed_size(&self) -> Result<u64> {
        combined_accessor(self.cdrh.compressed_size, &self.efs, |ei_data| ei_data.compressed_size)
    }

    pub fn find_ef(&self, efid: EFHID) -> Option<&EF> {
        self.efs.iter().find(|field| field.efh.efid == efid)
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
/// An end of central directory record.
pub struct EOCDR {
    pub eocdrh: EOCDRH,
    #[br(count = eocdrh.comment_length)]
    pub file_comment: ZipString,
}

/// A combined end of central directory record. This struct provides ZIP64-aware accessors.
#[derive(Clone, Debug)]
pub struct CEOCDR {
    pub eocdr: EOCDR,
    pub eocdr64: Option<EOCDR64H>,
    pub eocdl64: Option<EOCDL64H>,
}

impl CEOCDR {
    /// Returns whether this archive is a ZIP64 archive.
    pub fn is_zip64(&self) -> bool {
        let xor1 = self.eocdl64.is_some() && self.eocdr64.is_none();
        let xor2 = self.eocdl64.is_none() && self.eocdr64.is_some();

        if xor1 || xor2 {
            unreachable!("we should have returned an Err previously if we had an XOR situation");
        }

        self.eocdr64.is_some() && self.eocdl64.is_some()
    }

    /// A ZIP-64-aware accessor for the offset of the start of the central directory.
    pub fn cd_offset(&self) -> Result<u64> {
        combined_accessor_ecodr_u32(self.eocdr.eocdrh.cd_offset, self, |record| record.cd_offset)
    }

    /// A ZIP-64-aware accessor for the number of entries in the central directory.
    pub fn num_entries(&self) -> Result<u64> {
        combined_accessor_ecodr_u16(self.eocdr.eocdrh.num_of_entries, self, |record| record.num_entries)
    }

    /// A ZIP-64-aware accessor for the number of entries in the central directory on this disk.
    pub fn num_entries_on_disk(&self) -> Result<u64> {
        combined_accessor_ecodr_u16(self.eocdr.eocdrh.num_of_entries_this_disk, self, |record| record.num_entries_this_disk)
    }

    /// A ZIP-64-aware accessor for the size of the central directory.
    pub fn cd_size(&self) -> Result<u64> {
        combined_accessor_ecodr_u32(self.eocdr.eocdrh.cd_size, self, |record| record.cd_size)
    }

    /// A ZIP-64-aware accessor for the disk number of the central directory.
    pub fn disk_num(&self) -> Result<u32> {
        combined_accessor_ecodr_disk(self.eocdr.eocdrh.disk_num, self, |record| record.disk_num)
    }

    /// A ZIP-64-aware accessor for the disk number of the start of the central directory.
    pub fn disk_num_start(&self) -> Result<u32> {
        combined_accessor_ecodr_disk(self.eocdr.eocdrh.disk_num_start_of_cd, self, |record| record.disk_num_start_of_cd)
    }
}
