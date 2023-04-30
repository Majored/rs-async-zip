// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod builder;

use futures_util::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, SeekFrom};

use crate::entry::builder::ZipEntryBuilder;
use crate::error::{Result, ZipError};
use crate::spec::{
    attribute::AttributeCompatibility,
    consts::LFH_SIGNATURE,
    header::{ExtraField, LocalFileHeader},
    Compression,
};
use crate::ZipDateTime;

/// An immutable store of data about a ZIP entry.
///
/// This type cannot be directly constructed so instead, the [`ZipEntryBuilder`] must be used. Internally this builder
/// stores a [`ZipEntry`] so conversions between these two types via the [`From`] implementations will be
/// non-allocating.
#[derive(Clone, Debug)]
pub struct ZipEntry {
    pub(crate) filename: String,
    pub(crate) compression: Compression,
    #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
    pub(crate) compression_level: async_compression::Level,
    pub(crate) crc32: u32,
    pub(crate) uncompressed_size: u64,
    pub(crate) compressed_size: u64,
    pub(crate) attribute_compatibility: AttributeCompatibility,
    pub(crate) last_modification_date: ZipDateTime,
    pub(crate) internal_file_attribute: u16,
    pub(crate) external_file_attribute: u32,
    pub(crate) extra_fields: Vec<ExtraField>,
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
            #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
            compression_level: async_compression::Level::Default,
            crc32: 0,
            uncompressed_size: 0,
            compressed_size: 0,
            attribute_compatibility: AttributeCompatibility::Unix,
            last_modification_date: ZipDateTime::default(),
            internal_file_attribute: 0,
            external_file_attribute: 0,
            extra_fields: Vec::new(),
            comment: String::new(),
        }
    }

    /// Returns the entry's filename.
    ///
    /// ## Note
    /// This will return the raw filename stored during ZIP creation. If calling this method on entries retrieved from
    /// untrusted ZIP files, the filename should be sanitised before being used as a path to prevent [directory
    /// traversal attacks](https://en.wikipedia.org/wiki/Directory_traversal_attack).
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
    pub fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }

    /// Returns the entry's compressed size.
    pub fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// Returns the entry's attribute's host compatibility.
    pub fn attribute_compatibility(&self) -> AttributeCompatibility {
        self.attribute_compatibility
    }

    /// Returns the entry's last modification time & date.
    pub fn last_modification_date(&self) -> &ZipDateTime {
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
    pub fn extra_fields(&self) -> &[ExtraField] {
        &self.extra_fields
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

/// An immutable store of data about how a ZIP entry is stored within a specific archive.
///
/// Besides storing archive independent information like the size and timestamp it can also be used to query
/// information about how the entry is stored in an archive.
#[derive(Clone)]
pub struct StoredZipEntry {
    pub(crate) entry: ZipEntry,
    // pub(crate) general_purpose_flag: GeneralPurposeFlag,
    pub(crate) file_offset: u64,
}

impl StoredZipEntry {
    /// Returns a reference to the inner ZIP entry.
    pub fn entry(&self) -> &ZipEntry {
        &self.entry
    }

    /// Returns the offset in bytes to where the header of the entry starts.
    pub fn header_offset(&self) -> u64 {
        self.file_offset
    }

    /// Seek to the offset in bytes where the data of the entry starts.
    pub(crate) async fn seek_to_data_offset<R: AsyncRead + AsyncSeek + Unpin>(&self, mut reader: &mut R) -> Result<()> {
        // Seek to the header
        reader.seek(SeekFrom::Start(self.file_offset)).await?;

        // Check the signature
        let signature = {
            let mut buffer = [0; 4];
            reader.read_exact(&mut buffer).await?;
            u32::from_le_bytes(buffer)
        };

        match signature {
            LFH_SIGNATURE => (),
            actual => return Err(ZipError::UnexpectedHeaderError(actual, LFH_SIGNATURE)),
        };

        // Skip the local file header and trailing data
        let header = LocalFileHeader::from_reader(&mut reader).await?;
        let _filename = crate::base::read::io::read_string(&mut reader, header.file_name_length.into()).await?;
        let _extra_field = crate::base::read::io::read_bytes(&mut reader, header.extra_field_length.into()).await?;

        Ok(())
    }
}
