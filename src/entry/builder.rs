// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use chrono::{DateTime, Utc};
use crate::spec::compression::Compression;
use crate::spec::attribute::AttributeCompatibility;
use crate::entry::ZipEntry;

#[cfg(doc)]
use crate::entry::ext::ZipEntryBuilderExt;

use super::CompressionLevel;

/// A builder for [`ZipEntry`].
/// 
/// As with the built type, this builder is intended to solely provide access to the raw underlying data. Any
/// additional or more complex operations are provided within an extension trait, [`ZipEntryBuilderExt`].
pub struct ZipEntryBuilder(pub(crate) ZipEntry);

impl From<ZipEntry> for ZipEntryBuilder {
    fn from(entry: ZipEntry) -> Self {
        Self(entry)
    }
}

impl ZipEntryBuilder {
    /// Constructs a new builder which defines the raw underlying data of a ZIP entry.
    /// 
    /// A filename and compression method are needed to construct the builder as minimal parameters.
    pub fn new(filename: String, compression: Compression) -> Self {
        Self(ZipEntry::new(filename, compression))
    }

    /// Set the compression level.
    ///
    /// What the precise value means is implementation defined so it
    /// depends on what compression algorithm is used.
    pub fn set_compression_level(mut self, level: CompressionLevel) -> Self {
	self.0.compression_level = level.into_level();
	self
    }

    /// Sets the entry's attribute host compatibility.
    pub fn attribute_compatibility(mut self, compatibility: AttributeCompatibility) -> Self {
        self.0.attribute_compatibility = compatibility;
        self
    }

    /// Sets the entry's last modification date.
    pub fn last_modification_date(mut self, date: DateTime<Utc>) -> Self {
        self.0.last_modification_date = date;
        self
    }

    /// Sets the entry's internal file attribute.
    pub fn internal_file_attribute(mut self, attribute: u16) -> Self {
        self.0.internal_file_attribute = attribute;
        self
    }

    /// Sets the entry's external file attribute.
    pub fn external_file_attribute(mut self, attribute: u32) -> Self {
        self.0.external_file_attribute = attribute;
        self
    }

    /// Sets the entry's extra field data.
    pub fn extra_field(mut self, field: Vec<u8>) -> Self {
        self.0.extra_field = field;
        self
    }

    /// Sets the entry's file comment.
    pub fn comment(mut self, comment: String) -> Self {
        self.0.comment = comment;
        self
    }

    /// Consumes this builder and returns a final [`ZipEntry`].
    /// 
    /// This is equivalent to:
    /// ```
    /// # use async_zip::{ZipEntry, ZipEntryBuilder, Compression};
    /// #
    /// # let builder = ZipEntryBuilder::new(String::from("foo.bar"), Compression::Deflate);
    /// let entry: ZipEntry = builder.into();
    /// ```
    pub fn build(self) -> ZipEntry {
        self.into()
    }
}
