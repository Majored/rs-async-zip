// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use binrw::binrw;

use crate::spec::{extra::{EF, EFD, EFID, Zip64EI, efs}, headers1::{CDRH, EOCDL64H, EOCDR64H, EOCDRH, LFH}, string::ZipString};

// Constructing blocks from raw headers & bytes sequences (like filenames, comments, etc).
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
    #[br(count = lfh.file_name_length, args { utf8: lfh.flags.language_encoding_flag() })]
    /// This filename is insecure. Untrusted values may contain path traversal sequences or similar.
    pub insecure_file_name: ZipString,
    #[br(parse_with = efs, args(lfh.extra_field_length.into()))]
    pub efs: Vec<EF>,
}

impl LF {
    // TODO: Add a Info-ZIP-aware filename accessor that uses the

    /// A ZIP-64-aware accessor for the uncompressed size of the file.
    pub fn uncompressed_size(&self) -> u64 {
        combined_size(self.lfh.uncompressed_size, &self.efs, |ei_data| ei_data.uncompressed_size)
    }

    /// A ZIP-64-aware accessor for the compressed size of the file.
    pub fn compressed_size(&self) -> u64 {
        combined_size(self.lfh.compressed_size, &self.efs, |ei_data| ei_data.compressed_size)
    }
}

fn combined_size(zip32_size: u32, extra_fields: &[EF], accessor: impl Fn(&Zip64EI) -> u64) -> u64 {
    if zip32_size != u32::MAX {
        return zip32_size.into();
    }

    let zip64ei = extra_fields.iter().find(|field| {
        matches!(field.efh.efid, EFID::EI64)
    });

    if let Some(EF { efh: _, efd: EFD::Zip64EI(data) }) = zip64ei {
        return accessor(data);
    }

    unreachable!();
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// A central directory record. This struct provides ZIP64-aware accessors.
pub struct CDR {
    /// The central directory record header.
    pub cdrh: CDRH,
    #[br(count = cdrh.file_name_length, args { utf8: cdrh.flags.language_encoding_flag() })]
    /// This filename is insecure. Untrusted values may contain path traversal sequences or similar.
    pub insecure_file_name: ZipString,
    #[br(parse_with = efs, args(cdrh.extra_field_length.into()))]
    /// A list of extra fields.
    pub efs: Vec<EF>,
    #[br(count = cdrh.file_comment_length, args { utf8: cdrh.flags.language_encoding_flag() })]
    pub file_comment: ZipString,
}

impl CDR {
    /// A ZIP-64-aware accessor for the uncompressed size of the file.
    pub fn uncompressed_size(&self) -> u64 {
        combined_size(self.cdrh.uncompressed_size, &self.efs, |ei_data| ei_data.uncompressed_size)
    }

    /// A ZIP-64-aware accessor for the compressed size of the file.
    pub fn compressed_size(&self) -> u64 {
        combined_size(self.cdrh.compressed_size, &self.efs, |ei_data| ei_data.compressed_size)
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
/// An end of central directory record.
pub struct EOCDR {
    pub eocdrh: EOCDRH,
    // The end of central directory record has no flags of its own, so its comment is never
    // declared as UTF-8 and `ZipStringArgs::utf8` is left at its default.
    #[br(count = eocdrh.file_comm_length)]
    pub file_comment: ZipString,
}

/// A combined end of central directory record. This struct provides ZIP64-aware accessors.
#[derive(Clone, Debug)]
pub struct CombinedEOCDR {
    pub eocdr: EOCDR,
    pub eocdr64: Option<EOCDR64H>,
    pub eocdl64: Option<EOCDL64H>,
}

impl CombinedEOCDR {
    /// Returns whether this archive is a ZIP64 archive.
    pub fn is_zip64(&self) -> bool {
        // TODO: I don't think we'd ever have an XOR situation here.
        self.eocdr64.is_some() && self.eocdl64.is_some()
    }

    /// A ZIP-64-aware accessor for the offset of the start of the central directory.
    pub fn cd_offset(&self) -> u64 {
        if let Some(record) = &self.eocdr64 {
            return record.offset_of_start_of_directory;
        }

        return self.eocdr.eocdrh.cent_dir_offset.into();
    }

    /// A ZIP-64-aware accessor for the number of entries in the central directory.
    pub fn num_entries(&self) -> u64 {
        if let Some(record) = &self.eocdr64 {
            return record.num_entries_in_directory;
        }

        return self.eocdr.eocdrh.num_of_entries.into();
    }

    /// A ZIP-64-aware accessor for the number of entries in the central directory on this disk.
    pub fn num_entries_on_disk(&self) -> u64 {
        if let Some(record) = &self.eocdr64 {
            return record.num_entries_in_directory_on_disk;
        }

        return self.eocdr.eocdrh.num_of_entries_disk.into();
    }

    /// A ZIP-64-aware accessor for the size of the central directory.
    pub fn cd_size(&self) -> u64 {
        if let Some(record) = &self.eocdr64 {
            return record.directory_size;
        }

        return self.eocdr.eocdrh.size_cent_dir.into();
    }

    /// A ZIP-64-aware accessor for the disk number of the central directory.
    pub fn disk_num(&self) -> u32 {
        if let Some(record) = &self.eocdr64 {
            return record.disk_number;
        }

        return self.eocdr.eocdrh.disk_num.into();
    }

    /// A ZIP-64-aware accessor for the disk number of the start of the central directory.
    pub fn disk_num_start(&self) -> u32 {
        if let Some(record) = &self.eocdr64 {
            return record.disk_number_start_of_cd;
        }

        return self.eocdr.eocdrh.start_cent_dir_disk.into();
    }
}
