// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which holds relevant error reporting structures/types.

/// A Result type alias over ZipError to minimise repetition.
pub type Result<V> = std::result::Result<V, ZipError>;

/// An enum of possible errors and their descriptions.
#[derive(Debug)]
pub enum ZipError {
    UnexpectedHeaderError(u32, u32),
    UnsupportedCompressionError(u16),
    UpstreamReadError(std::io::Error),
    FeatureNotSupported(&'static str),
    CRC32CheckError,
    EntryIndexOutOfBounds,
}

impl ZipError {
    pub fn description(&self) -> String {
        match self {
            ZipError::UnexpectedHeaderError(actual, expected) => {
                format!("Encountered an unexpected header (actual: {:#x}, expected: {:#x}).", actual, expected)
            }
            ZipError::UnsupportedCompressionError(actual) => format!("{} is not a supported compression type.", actual),
            ZipError::UpstreamReadError(inner) => format!("An upstream reader returned an error: '{:?}'.", inner),
            ZipError::FeatureNotSupported(feature) => format!("Feature not currently supported: '{}'.", feature),
            ZipError::CRC32CheckError => format!("A computed CRC32 value did not match the expected value."),
            ZipError::EntryIndexOutOfBounds => format!("Entry index was out of bounds."),
        }
    }
}

impl From<std::io::Error> for ZipError {
    fn from(err: std::io::Error) -> Self {
        ZipError::UpstreamReadError(err)
    }
}
