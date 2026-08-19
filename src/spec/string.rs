// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use binrw::helpers::count;
use binrw::io::{Read, Seek, Write};
use binrw::{BinRead, BinResult, BinWrite, Endian, NamedArgs};

/// A string as stored within a ZIP archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZipString {
    UTF8(String),
    Raw(Vec<u8>),
}

impl ZipString {
    /// Constructs a string from its raw bytes, given whether the record declared them as UTF-8.
    pub fn from_bytes(raw: Vec<u8>, declared_utf8: bool) -> Self {
        if !declared_utf8 && !raw.is_ascii() {
            return Self::Raw(raw);
        }

        match String::from_utf8(raw) {
            Ok(string) => Self::UTF8(string),
            Err(error) => Self::Raw(error.into_bytes()),
        }
    }

    /// Returns the raw bytes of this string, as they appear within the archive.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::UTF8(string) => string.as_bytes(),
            Self::Raw(raw) => raw,
        }
    }

    /// Returns this string as a string slice, if its encoding is known to be UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::UTF8(string) => Some(string),
            Self::Raw(_) => None,
        }
    }
}

impl PartialEq<[u8]> for ZipString {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl PartialEq<&[u8]> for ZipString {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_bytes() == *other
    }
}

impl PartialEq<str> for ZipString {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl From<String> for ZipString {
    fn from(value: String) -> Self {
        Self::UTF8(value)
    }
}

impl From<&str> for ZipString {
    fn from(value: &str) -> Self {
        Self::UTF8(value.to_owned())
    }
}

/// The arguments needed to read a [`ZipString`].
#[derive(Clone, NamedArgs)]
pub struct ZipStringArgs {
    pub count: usize,
    #[named_args(default = false)]
    pub utf8: bool,
}

impl BinRead for ZipString {
    type Args<'a> = ZipStringArgs;

    fn read_options<R: Read + Seek>(reader: &mut R, endian: Endian, args: Self::Args<'_>) -> BinResult<Self> {
        let raw: Vec<u8> = count(args.count)(reader, endian, ())?;
        Ok(Self::from_bytes(raw, args.utf8))
    }
}

impl BinWrite for ZipString {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(&self, writer: &mut W, endian: Endian, _: ()) -> BinResult<()> {
        self.as_bytes().write_options(writer, endian, ())
    }
}
