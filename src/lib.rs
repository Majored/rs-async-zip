// Copyright (c) 2021-2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.
//!
//! ## Features
//! - Asynchronous design powered by tokio.
//! - Support for Stored, Deflate, bzip2, LZMA, zstd, and xz compression methods.
//! - Various different reading approaches (seek, stream, filesystem, in-memory buffer).
//! - Support for writing complete data (u8 slices) or stream writing using data descriptors.
//! - Aims for reasonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.
//!
//! [Read more.](https://github.com/Majored/rs-async-zip)

pub mod error;
pub mod read;
pub mod write;

pub(crate) mod entry;
pub(crate) mod file;
pub(crate) mod spec;
pub(crate) mod utils;

#[cfg(test)]
pub(crate) mod tests;

pub use crate::spec::attribute::AttributeCompatibility;
pub use crate::spec::compression::{Compression, DeflateOption};
pub use crate::spec::date::ZipDateTime;

pub use crate::entry::{builder::ZipEntryBuilder, StoredZipEntry, ZipEntry};
pub use crate::file::{builder::ZipFileBuilder, ZipFile};
