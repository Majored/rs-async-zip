// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod ext;
pub mod builder;

use chrono::{DateTime, Utc};
use crate::spec::compression::Compression;
use crate::spec::attribute::AttributeCompatibility;
use crate::entry::builder::EntryBuilder;

#[cfg(doc)]
use crate::entry::ext::EntryExt;

/// Stores information about a ZIP entry.
/// 
/// # Builder pattern
/// Each [`Entry`] is immutable for interoperability between the reading and writing modules of this crate. Therefore,
/// to create or mutate an existing entry, the [`EntryBuilder`] builder must be used.
/// 
/// Non-allocating conversions between these two structures can be achieved via the [`From`] implementations.
/// 
/// # Extension trait
/// TBC. [`EntryExt`]
/// 
#[derive(Clone)]
pub struct Entry {
    pub(crate) filename: String,
    pub(crate) compression: Compression,
    pub(crate) attribute_compatibility: AttributeCompatibility,
    pub(crate) last_modification_date: DateTime<Utc>,
    pub(crate) internal_file_attribute: u16,
    pub(crate) external_file_attribute: u32,
    pub(crate) extra_field: Vec<u8>,
    pub(crate) comment: String,
}

impl From<EntryBuilder> for Entry {
    fn from(builder: EntryBuilder) -> Self {
        let attribute_compatibility = builder.attribute_compatibility.unwrap_or(AttributeCompatibility::Unix);
        let last_modification_date = builder.last_modification_date.unwrap_or(Utc::now());
        let internal_file_attribute = builder.internal_file_attribute.unwrap_or(0);
        let external_file_attribute = builder.external_file_attribute.unwrap_or(0);
        let extra_field = builder.extra_field.unwrap_or(Vec::new());
        let comment = builder.comment.unwrap_or(String::new());

        Self {
            filename: builder.filename,
            compression: builder.compression,
            attribute_compatibility,
            last_modification_date,
            internal_file_attribute,
            external_file_attribute,
            extra_field,
            comment,
        }
    }
}

impl Entry {
    /// Returns the entry's filename.
    /// 
    /// # Note
    /// This will return the raw filename stored during ZIP creation. If calling this method on entries retrieved from
    /// untrusted ZIP files, the filename should be sanitised before being used as a path to prevent [directory
    /// travesal attacks](https://en.wikipedia.org/wiki/Directory_traversal_attack).
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Returns the entry's compression method.
    pub fn compression(&self) -> Compression {
        self.compression
    }

    /// Returns the entry's attribute's host compatibility.
    pub fn attribute_compatibility(&self) -> AttributeCompatibility {
        self.attribute_compatibility
    }

    /// Returns the entry's last modification time & date.
    pub fn last_modification_date(&self) -> &DateTime<Utc> {
        &self.last_modification_date
    }

    /// Returns the entry's internal file attribute.
    pub fn internal_file_attribute(&self) -> u16 {
        self.internal_file_attribute
    }

    /// Returns the entry's external file attribute
    pub fn external_file_attribute(&self) -> u32 {
        self.external_file_attribute
    }

    /// Returns the entry's extra field data.
    pub fn extra_field(&self) -> &[u8] {
        &self.extra_field
    }

    /// Returns the entry's file comment.
    pub fn comment(&self) -> &str {
        &self.comment
    }
}
