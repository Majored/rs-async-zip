// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub type Result<V> = std::result::Result<V, ZipError>;

#[derive(Debug)]
pub enum ZipError {
    LocalFileHeaderError(u32),
    UnsupportedCompressionError(u16),
    ReadFailed,
    FeatureNotSupported(&'static str),
}

impl ZipError {
    pub fn description(&self) -> String {
        match self {
            ZipError::LocalFileHeaderError(actual) => {
                format!("{} != {} or any supported ZIP delimiter.", actual, crate::delim::LFHD)
            }
            ZipError::UnsupportedCompressionError(actual) => format!("{} is not a supported compression type.", actual),
            ZipError::ReadFailed => format!("Read failed."),
            ZipError::FeatureNotSupported(feature) => format!("Feature not currently supported: '{}'.", feature),
        }
    }
}

impl From<tokio::io::Error> for ZipError {
    fn from(_err: tokio::io::Error) -> Self {
        ZipError::ReadFailed
    }
}
