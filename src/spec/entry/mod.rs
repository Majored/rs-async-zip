pub mod ext;

use chrono::{DateTime, Utc};
use crate::spec::compression::Compression;
use crate::spec::attribute::AttributeCompatibility;

pub struct Entry {
    filename: String,
    compression: Compression,
    attribute_compatibility: AttributeCompatibility,
    last_modification_date: DateTime<Utc>,
    internal_file_attribute: u16,
    external_file_attribute: u32,
    extra_field: Vec<u8>,
    comment: String,
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

pub struct EntryBuilder {
    filename: String,
    compression: Compression,
    attribute_compatibility: Option<AttributeCompatibility>,
    last_modification_date: Option<DateTime<Utc>>,
    internal_file_attribute: Option<u16>,
    external_file_attribute: Option<u32>,
    extra_field: Option<Vec<u8>>,
    comment: Option<String>,
}

impl EntryBuilder {
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

    /// 
    pub fn attribute_compatibility(mut self, compatibility: AttributeCompatibility) -> Self {
        self.attribute_compatibility = Some(compatibility);
        self
    }

    ///
    pub fn last_modification_date(mut self, date: DateTime<Utc>) -> Self {
        self.last_modification_date = Some(date);
        self
    }

    /// 
    pub fn internal_file_attribute(mut self, attribute: u16) -> Self {
        self.internal_file_attribute = Some(attribute);
        self
    }

    /// 
    pub fn external_file_attribute(mut self, attribute: u32) -> Self {
        self.external_file_attribute = Some(attribute);
        self
    }

    /// 
    pub fn extra_field(mut self, field: Vec<u8>) -> Self {
        self.extra_field = Some(field);
        self
    }

    /// 
    pub fn comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// 
    pub fn build(self) -> Entry {
        self.into()
    }
}

