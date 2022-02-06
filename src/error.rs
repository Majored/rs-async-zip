// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which holds relevant error reporting structures/types.

use thiserror::Error;

/// A Result type alias over ZipError to minimise repetition.
pub type Result<V> = std::result::Result<V, ZipError>;

/// An enum of possible errors and their descriptions.
#[derive(Debug, Error)]
pub enum ZipError {
    #[error("Encountered an unexpected header (actual: {0:#x}, expected: {1:#x}).")]
    UnexpectedHeaderError(u32, u32),
    #[error("{0} is not a supported compression type.")]
    UnsupportedCompressionError(u16),
    #[error("An upstream reader returned an error: '{0:?}'.")]
    UpstreamReadError(#[from] std::io::Error),
    #[error("Feature not currently supported: '{0}'.")]
    FeatureNotSupported(&'static str),
    #[error("A computed CRC32 value did not match the expected value.")]
    CRC32CheckError,
    #[error("Entry index was out of bounds.")]
    EntryIndexOutOfBounds,
}
