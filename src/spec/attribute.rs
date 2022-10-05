// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::{Result, ZipError};

/// An attribute host compatibility supported by this crate.
#[non_exhaustive]
#[derive(Copy, Clone)]
pub enum AttributeCompatibility {
    Unix
}

impl TryFrom<u16> for AttributeCompatibility {
    type Error = ZipError;

    fn try_from(value: u16) -> Result<Self> {
        match value {
            3 => Ok(AttributeCompatibility::Unix),
            _ => Err(ZipError::UnsupportedAttributeCompatibility(value))
        }
    }
}

impl From<&AttributeCompatibility> for u16 {
    fn from(compatibility: &AttributeCompatibility) -> Self {
        match compatibility {
            AttributeCompatibility::Unix => 3
        }
    }
}

impl From<AttributeCompatibility> for u16 {
    fn from(compatibility: AttributeCompatibility) -> Self {
        (&compatibility).into()
    }
}