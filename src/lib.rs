// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! # async_zip
//!
//! An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.
//!
//! ## Features
//! - Asynchronous design powered by tokio.
//! - Support for Stored, Deflate, bzip2, LZMA, zstd, and xz compression methods.
//! - Various different reading approaches (seek, stream, filesystem, in-memory buffer).
//! - Support for writing a complete data (u8 slices) or stream writing using data descriptors (unimplemented).
//! - Aims for reasonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.
//!
//! [Read more.](https://github.com/Majored/rs-async-zip)

#[macro_use]
pub(crate) mod header;
pub(crate) mod delim;
pub mod error;
pub mod read;
pub(crate) mod utils;
pub mod write;

pub(crate) use array_push;

use error::{Result, ZipError};

/// A compression method supported by this crate.
#[derive(Clone)]
pub enum Compression {
    Stored,
    Deflate,
    Bz,
    Lzma,
    Zstd,
    Xz,
}

impl Compression {
    /// Convert a supported compression method into its relevant u16 stored with little endianness.
    pub fn to_u16(&self) -> u16 {
        match self {
            Compression::Stored => 0,
            Compression::Deflate => 8,
            Compression::Bz => 12,
            Compression::Lzma => 14,
            Compression::Zstd => 93,
            Compression::Xz => 95,
        }
    }

    /// Convert a u16 stored with little endianness into a supported compression method.
    pub fn from_u16(value: u16) -> Result<Compression> {
        match value {
            0 => Ok(Compression::Stored),
            8 => Ok(Compression::Deflate),
            12 => Ok(Compression::Bz),
            14 => Ok(Compression::Lzma),
            93 => Ok(Compression::Zstd),
            95 => Ok(Compression::Xz),
            _ => Err(ZipError::UnsupportedCompressionError(value)),
        }
    }
}
