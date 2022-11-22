// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which holds relevant error reporting structures/types.

use thiserror::Error;

/// A Result type alias over ZipError to minimise repetition.
pub type Result<V> = std::result::Result<V, ZipError>;

/// An enum of possible errors and their descriptions.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ZipError {
    #[error("feature not supported: '{0}'")]
    FeatureNotSupported(&'static str),
    #[error("compression not supported: {0}")]
    CompressionNotSupported(u16),
    #[error("host attribute compatibility not supported: {0}")]
    AttributeCompatibilityNotSupported(u16),
    #[error("attempted to read a ZIP64 file whilst on a 32-bit target")]
    TargetZip64NotSupported,

    #[error("unable to locate the end of central directory record")]
    UnableToLocateEOCDR,

    #[error("an upstream reader returned an error: {0}")]
    UpstreamReadError(#[from] std::io::Error),
    #[error("a computed CRC32 value did not match the expected value")]
    CRC32CheckError,
    #[error("entry index was out of bounds")]
    EntryIndexOutOfBounds,
    #[error("Encountered an unexpected header (actual: {0:#x}, expected: {1:#x}).")]
    UnexpectedHeaderError(u32, u32),
}
