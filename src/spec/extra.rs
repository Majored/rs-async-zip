// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of extra-field specific headers and data structures.

use std::io::{Cursor, Seek};

use binrw::helpers::{count, until_eof};
use binrw::{binrw, BinRead, BinResult, BinWrite, Endian};

use crate::spec::KnownSize;

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// An extra field header.
pub struct EFH {
    pub efid: EFHID,
    pub data_size: u16,
}

impl KnownSize for EFH {
    const SIZE: usize = 4;
}

#[binrw]
#[brw(little)]
#[brw(repr = u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// An extra field header ID.
pub enum EFHID {
    EI64,
    IZUC,
    IZUP,
    Other(u16),
}

impl EFHID {
    const KNOWN: &'static [(u16, EFHID)] = &[
        (0x0001, Self::EI64),
        (0x6375, Self::IZUC),
        (0x7075, Self::IZUP),
    ];
}

impl From<u16> for EFHID {
    fn from(value: u16) -> Self {
        match Self::KNOWN.iter().find(|(tag, _)| *tag == value) {
            Some((_, id)) => *id,
            None => Self::Other(value),
        }
    }
}

impl From<&EFHID> for u16 {
    fn from(id: &EFHID) -> Self {
        match id {
            EFHID::Other(v) => *v,
            known => EFHID::KNOWN.iter().find(|(_, id)| id == known).map(|(tag, _)| *tag).unwrap(),
        }
    }
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// An extra field.
pub struct EF {
    pub efh: EFH,
    #[br(parse_with = ef_data, args(efh.efid, efh.data_size))]
    pub efd: EFD,
}

#[derive(Clone, Debug)]
// An extra field's data, which may be one of several known types or an unknown type.
pub enum EFD {
    EI64(EI64),
    IZUF(IZUF),
    IZUC(IZUC),
    Unknown(Vec<u8>),
}

#[binrw]
#[brw(little)]
#[br(import(size: u16))]
#[derive(Clone, Debug)]
/// A ZIP64 extended information extra field data variant.
///
/// Only the two sizes are mandatory; the trailing fields are present or absent depending
/// on which of the header's 32-bit fields were saturated, so their presence is driven by
/// the field's declared size.
pub struct EI64 {
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
/// An Info-ZIP unicode extra field header data variant.
pub struct IZUCH {
    pub version: u8,
    pub crc32: u32,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// An Info-ZIP unicode comment extra field data variant.
pub struct IZUC {
    pub izuch: IZUCH,
    // No length arithmetic needed: `ef_data` hands us a reader bounded to this field.
    #[br(parse_with = until_eof, try_map = String::from_utf8)]
    #[bw(map = |comment: &String| comment.as_bytes().to_vec())]
    pub comment: String,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug)]
/// An Info-ZIP unicode path extra field data variant.
pub struct IZUF {
    pub izuch: IZUCH,
    #[br(parse_with = until_eof, try_map = String::from_utf8)]
    #[bw(map = |file_name: &String| file_name.as_bytes().to_vec())]
    pub insecure_file_name: String,
}

/// Reads extra fields until `len` bytes have been consumed.
///
/// `binrw::helpers::until_eof` is deliberately not used here as it treats an
/// `UnexpectedEof` from the item parser as the end of the collection, which would silently
/// truncate the list for a field which lies about its `data_size`.
#[binrw::parser(reader, endian)]
pub(crate) fn efs(len: u64) -> BinResult<Vec<EF>> {
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
fn ef_data(tag: EFHID, size: u16) -> BinResult<EFD> {
    let raw: Vec<u8> = count(size.into())(reader, endian, ())?;

    Ok(match tag {
        EFHID::EI64 => {
            EFD::EI64(EI64::read_options(&mut Cursor::new(&raw), endian, (size,))?)
        },
        EFHID::IZUC => {
            EFD::IZUC(IZUC::read_options(&mut Cursor::new(&raw), endian, ())?)
        }
        EFHID::IZUP => {
            EFD::IZUF(IZUF::read_options(&mut Cursor::new(&raw), endian, ())?)
        }
        EFHID::Other(_) => EFD::Unknown(raw),
    })
}

impl BinWrite for EFD {
    type Args<'a> = ();

    fn write_options<W: binrw::io::Write + Seek>(&self, writer: &mut W, endian: Endian, _: ()) -> BinResult<()> {
        match self {
            Self::EI64(field) => field.write_options(writer, endian, ()),
            Self::IZUF(field) => field.write_options(writer, endian, ()),
            Self::IZUC(field) => field.write_options(writer, endian, ()),
            Self::Unknown(raw) => raw.write_options(writer, endian, ()),
        }
    }
}
