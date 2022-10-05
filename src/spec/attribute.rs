use crate::error::ZipError;

#[non_exhaustive]
pub enum AttributeCompatibility {
    Unix
}

impl TryFrom<u16> for AttributeCompatibility {
    type Error = ZipError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(AttributeCompatibility::Unix),
            _ => Err(ZipError::UnsupportedAttributeCompatibility(value))
        }
    }
}

impl From<&AttributeCompatibility> for u16 {
    fn from(AttributeCompatibility: &AttributeCompatibility) -> Self {
        match AttributeCompatibility {
            AttributeCompatibility::Unix => 3
        }
    }
}