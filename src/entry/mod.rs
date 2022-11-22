// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod builder;

use crate::entry::builder::ZipEntryBuilder;
use crate::spec::attribute::AttributeCompatibility;
use crate::spec::compression::Compression;
use crate::spec::consts::{LFH_LENGTH, SIGNATURE_LENGTH};
use crate::spec::header::GeneralPurposeFlag;
use chrono::{DateTime, Utc};

/// An immutable store of data about a ZIP entry.
///
/// This type cannot be directly constructed so instead, the [`ZipEntryBuilder`] must be used. Internally this builder
/// stores a [`ZipEntry`] so conversions between these two types via the [`From`] implementations will be
/// non-allocating.
#[derive(Clone)]
pub struct ZipEntry {
    pub(crate) filename: String,
    pub(crate) compression: Compression,
    pub(crate) compression_level: async_compression::Level,
    pub(crate) crc32: u32,
    pub(crate) uncompressed_size: u32,
    pub(crate) compressed_size: u32,
    pub(crate) attribute_compatibility: AttributeCompatibility,
    pub(crate) last_modification_date: DateTime<Utc>,
    pub(crate) internal_file_attribute: u16,
    pub(crate) external_file_attribute: u32,
    pub(crate) extra_field: Vec<u8>,
    pub(crate) comment: String,
}

impl From<ZipEntryBuilder> for ZipEntry {
    fn from(builder: ZipEntryBuilder) -> Self {
        builder.0
    }
}

impl ZipEntry {
    pub(crate) fn new(filename: String, compression: Compression) -> Self {
        ZipEntry {
            filename,
            compression,
            compression_level: async_compression::Level::Default,
            crc32: 0,
            uncompressed_size: 0,
            compressed_size: 0,
            attribute_compatibility: AttributeCompatibility::Unix,
            last_modification_date: Utc::now(),
            internal_file_attribute: 0,
            external_file_attribute: 0,
            extra_field: Vec::new(),
            comment: String::new(),
        }
    }

    /// Returns the entry's filename.
    ///
    /// ## Note
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

    /// Returns the entry's CRC32 value.
    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    /// Returns the entry's uncompressed size.
    pub fn uncompressed_size(&self) -> u32 {
        self.uncompressed_size
    }

    /// Returns the entry's compressed size.
    pub fn compressed_size(&self) -> u32 {
        self.compressed_size
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

    /// Returns the entry's integer-based UNIX permissions.
    ///
    /// # Note
    /// This will return None if the attribute host compatibility is not listed as Unix.
    pub fn unix_permissions(&self) -> Option<u16> {
        if !matches!(self.attribute_compatibility, AttributeCompatibility::Unix) {
            return None;
        }

        Some(((self.external_file_attribute) >> 16) as u16)
    }

    /// Returns whether or not the entry represents a directory.
    pub fn dir(&self) -> bool {
        self.filename.ends_with('/')
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct ZipEntryMeta {
    pub(crate) general_purpose_flag: GeneralPurposeFlag,
    pub(crate) file_offset: u64,
}

/// Stores information about a Zip entry inside of an archive. Besides storing archive independant
/// information like the size and timestamp it can also be used to query information about how the
/// entry is stored in an archive.
#[derive(Clone)]
pub struct StoredZipEntry {
    pub entry: ZipEntry,
    pub(crate) meta: ZipEntryMeta,
}

impl StoredZipEntry {
    /// Returns the offset in bytes from where the data of the entry starts.
    pub fn data_offset(&self) -> u64 {
        let header_length = SIGNATURE_LENGTH + LFH_LENGTH;
        let trailing_length = self.entry.comment().as_bytes().len()
            + self.entry.extra_field().len()
            + self.entry.filename.as_bytes().len();

        self.meta.file_offset + (header_length as u64) + (trailing_length as u64)
    }
}
