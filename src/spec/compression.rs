// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};

/// A compression method supported by this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    Stored,
    #[cfg(feature = "deflate")]
    Deflate,
    #[cfg(feature = "bzip2")]
    Bz,
    #[cfg(feature = "lzma")]
    Lzma,
    #[cfg(feature = "zstd")]
    Zstd,
    #[cfg(feature = "xz")]
    Xz,
}

impl Compression {
    // Convert a supported compression method into its relevant u16 stored with little endianness.
    // https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#445
    pub fn to_u16(&self) -> u16 {
        match self {
            Compression::Stored => 0,
            #[cfg(feature = "deflate")]
            Compression::Deflate => 8,
            #[cfg(feature = "bzip2")]
            Compression::Bz => 12,
            #[cfg(feature = "lzma")]
            Compression::Lzma => 14,
            #[cfg(feature = "zstd")]
            Compression::Zstd => 93,
            #[cfg(feature = "xz")]
            Compression::Xz => 95,
        }
    }

    // Convert a u16 stored with little endianness into a supported compression method.
    // https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#445
    pub fn from_u16(value: u16) -> Result<Compression> {
        match value {
            0 => Ok(Compression::Stored),
            #[cfg(feature = "deflate")]
            8 => Ok(Compression::Deflate),
            #[cfg(feature = "bzip2")]
            12 => Ok(Compression::Bz),
            #[cfg(feature = "lzma")]
            14 => Ok(Compression::Lzma),
            #[cfg(feature = "zstd")]
            93 => Ok(Compression::Zstd),
            #[cfg(feature = "xz")]
            95 => Ok(Compression::Xz),
            _ => Err(ZipError::UnsupportedCompressionError(value)),
        }
    }
}
