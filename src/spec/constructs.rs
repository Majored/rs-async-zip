// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::{Cursor, Seek};

use binrw::helpers::{count, until_eof};
use binrw::{binrw, BinRead, BinResult, BinWrite, Endian};

use crate::spec::headers1::{ExtraFieldHeaderId, CDRH, EFH, EOCDL64H, EOCDR64H, EOCDRH, LFH};
use crate::tests::read::zip64;

// Constructing blocks from raw headers & bytes sequences (like filenames, comments, etc).
//
// Every construct here is parsed by binrw from an in-memory buffer. The lengths of the
// variable-length parts are always stated by the fixed-size header which precedes them,
// so the async caller only needs to know how many bytes to pull before handing the whole
// record over (see `headers1::read_record`).

#[binrw]
#[brw(little)]
#[derive(Debug)]
// Local file
pub struct LF {
    pub lfh: LFH,
    #[br(count = lfh.file_name_length)]
    pub file_name: Vec<u8>,
    #[br(parse_with = efs, args(lfh.extra_field_length.into()))]
    pub extra_fields: Vec<EF>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
// Central directory record
pub struct CDR {
    pub cdrh: CDRH,
    #[br(count = cdrh.file_name_length)]
    pub file_name: Vec<u8>,
    #[br(parse_with = efs, args(cdrh.extra_field_length.into()))]
    pub extra_fields: Vec<EF>,
    #[br(count = cdrh.file_comment_length)]
    pub file_comment: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
// End of central directory record
pub struct EOCDR {
    pub eocdrh: EOCDRH,
    #[br(count = eocdrh.file_comm_length)]
    pub file_comment: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
// Extra field
pub struct EF {
    pub efh: EFH,
    #[br(parse_with = ef_data, args(efh.tag, efh.data_size))]
    pub data: EFData,
}

#[derive(Clone, Debug)]
pub enum EFData {
    Zip64EI(Zip64EI),
    UnicodeFilename(UnicodeFilename),
    UnicodeComment(UnicodeComment),
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
#[br(import(size: u16))]
#[derive(Clone, Debug)]
/// ZIP64 extended information extra field
///
/// Only the two sizes are mandatory; the trailing fields are present or absent depending
/// on which of the header's 32-bit fields were saturated, so their presence is driven by
/// the field's declared size.
pub struct Zip64EI {
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    #[br(if(size >= 24))]
    pub relative_offset: Option<u64>,
    #[br(if(size >= 28))]
    pub disk_number_start: Option<u32>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// Info-ZIP unicode extra field header
pub struct UCH {
    pub version: u8,
    pub crc32: u32,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// Info-ZIP unicode comment extra field
pub struct UnicodeComment {
    pub uch: UCH,
    // No length arithmetic needed: `ef_data` hands us a reader bounded to this field.
    #[br(parse_with = until_eof, try_map = String::from_utf8)]
    #[bw(map = |comment: &String| comment.as_bytes().to_vec())]
    pub comment: String,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// Info-ZIP unicode path extra field
pub struct UnicodeFilename {
    pub uch: UCH,
    #[br(parse_with = until_eof, try_map = String::from_utf8)]
    #[bw(map = |file_name: &String| file_name.as_bytes().to_vec())]
    pub file_name: String,
}

/// Reads extra fields until `len` bytes have been consumed.
///
/// `binrw::helpers::until_eof` is deliberately not used here as it treats an
/// `UnexpectedEof` from the item parser as the end of the collection, which would silently
/// truncate the list for a field which lies about its `data_size`.
#[binrw::parser(reader, endian)]
fn efs(len: u64) -> BinResult<Vec<EF>> {
    let end = reader.stream_position()? + len;
    let mut fields = Vec::new();

    while reader.stream_position()? < end {
        fields.push(EF::read_options(reader, endian, ())?);
    }

    // A field whose `data_size` reaches past the end of the block would otherwise be
    // reported as whatever the following construct failed to parse.
    let position = reader.stream_position()?;
    if position != end {
        return Err(binrw::Error::AssertFail {
            pos: position,
            message: format!("extra field overran its block by {} byte(s)", position - end),
        });
    }

    Ok(fields)
}

#[binrw::parser(reader, endian)]
fn ef_data(tag: ExtraFieldHeaderId, size: u16) -> BinResult<EFData> {
    let raw: Vec<u8> = count(size.into())(reader, endian, ())?;

    Ok(match tag {
        ExtraFieldHeaderId::EI64 => {
            EFData::Zip64EI(Zip64EI::read_options(&mut Cursor::new(&raw), endian, (size,))?)
        },
        ExtraFieldHeaderId::IZUC => {
            EFData::UnicodeComment(UnicodeComment::read_options(&mut Cursor::new(&raw), endian, ())?)
        }
        ExtraFieldHeaderId::IZUP => {
            EFData::UnicodeFilename(UnicodeFilename::read_options(&mut Cursor::new(&raw), endian, ())?)
        }
        ExtraFieldHeaderId::Other(_) => EFData::Unknown(raw),
    })
}

impl BinWrite for EFData {
    type Args<'a> = ();

    fn write_options<W: binrw::io::Write + Seek>(&self, writer: &mut W, endian: Endian, _: ()) -> BinResult<()> {
        match self {
            Self::Zip64EI(field) => field.write_options(writer, endian, ()),
            Self::UnicodeFilename(field) => field.write_options(writer, endian, ()),
            Self::UnicodeComment(field) => field.write_options(writer, endian, ()),
            Self::Unknown(raw) => raw.write_options(writer, endian, ()),
        }
    }
}
