// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use chrono::{DateTime, Utc};
use crate::spec::compression::Compression;
use crate::spec::attribute::AttributeCompatibility;
use crate::entry::ZipEntry;

/// A ZIP entry builder.
pub struct ZipEntryBuilder {
    pub(crate) filename: String,
    pub(crate) compression: Compression,
    pub(crate) attribute_compatibility: Option<AttributeCompatibility>,
    pub(crate) last_modification_date: Option<DateTime<Utc>>,
    pub(crate) internal_file_attribute: Option<u16>,
    pub(crate) external_file_attribute: Option<u32>,
    pub(crate) extra_field: Option<Vec<u8>>,
    pub(crate) comment: Option<String>,
}

impl From<ZipEntry> for ZipEntryBuilder {
    fn from(entry: ZipEntry) -> Self {
        Self {
            filename: entry.filename,
            compression: entry.compression,
            attribute_compatibility: Some(entry.attribute_compatibility),
            last_modification_date: Some(entry.last_modification_date),
            internal_file_attribute: Some(entry.internal_file_attribute),
            external_file_attribute: Some(entry.external_file_attribute),
            extra_field: Some(entry.extra_field),
            comment: Some(entry.comment),
        }
    }
}

impl ZipEntryBuilder {
    /// Constructs a new builder which defines the properties of a writable ZIP entry.
    /// 
    /// A filename and compression method are needed to construct the builder as minimal parameters.
    pub fn new(filename: String, compression: Compression) -> Self {
        let attribute_compatibility = None;
        let last_modification_date = None;
        let internal_file_attribute = None;
        let external_file_attribute = None;
        let extra_field = None;
        let comment = None;

        Self {
            filename,
            compression,
            attribute_compatibility,
            last_modification_date,
            internal_file_attribute,
            external_file_attribute,
            extra_field,
            comment,
        }
    }

    /// Sets the entry's attribute host compatibility.
    pub fn attribute_compatibility(mut self, compatibility: AttributeCompatibility) -> Self {
        self.attribute_compatibility = Some(compatibility);
        self
    }

    /// Sets the entry's last modification date.
    pub fn last_modification_date(mut self, date: DateTime<Utc>) -> Self {
        self.last_modification_date = Some(date);
        self
    }

    /// Sets the entry's internal file attribute.
    pub fn internal_file_attribute(mut self, attribute: u16) -> Self {
        self.internal_file_attribute = Some(attribute);
        self
    }

    /// Sets the entry's external file attribute.
    pub fn external_file_attribute(mut self, attribute: u32) -> Self {
        self.external_file_attribute = Some(attribute);
        self
    }

    /// Sets the entry's extra field data.
    pub fn extra_field(mut self, field: Vec<u8>) -> Self {
        self.extra_field = Some(field);
        self
    }

    /// Sets the entry's file comment.
    pub fn comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Consumes this builder and returns a final [`Entry`].
    /// 
    /// # Equivalent code
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