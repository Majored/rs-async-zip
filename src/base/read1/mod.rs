// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of ZIP archive readers which vary based on their seeking vs streaming capabilities.
//! 
//! # AsyncBufRead requirement
//! All readers require an [`AsyncBufRead`] implementer as their underlying source.
//! 
//! ## Seek
//! The seek reader acts over a single [`AsyncSeek`] reader.
//! 
//! ### Advantages
//! - Can perform out-of-order file reads.
//! - Reads and uses the central directory as the source of truth for file metadata.
//! - Can perform validation of local file headers against the central directory.
//! - Can perform validation of the central directory itself.
//! - Can perform concurrent/parallel file reads when using a factory.
//! 
//! ### Limitations
//! - The underlying reader must implement [`AsyncSeek`] (or the tokio equivalent).
//! - File reads must be sequential (ie. one at a time) unless using a factory.
//! - [`ZipFileReader`] does not support seeking, so nested ZIPs must be opened with [`stream`].
//! 
//! ## Stream
//! The stream reader acts over a single non-[`AsyncSeek`] reader. Support for streaming across the industry is limited.
//! This is because it must read the archive in-order using local file headers which brings its own set of limitations.
//! 
//! Consider whether you truly need to stream a ZIP archive. In most cases, saving the stream to disk and using the seek
//! reader is a better option.
//! 
//! ### Advantages
//! - Operating in low-memory environments. You control buffering, we only read headers in one-by-one.
//! 
//! ### Limitations
//! - The inability to read ZIP entries using the combination of a data descriptor and the Stored compression method.
//! - No file comment being available (defaults to an empty string).
//! - No internal or external file attributes being available (defaults to 0).
//! - The extra field data potentially being inconsistent with what’s stored in the central directory.
//! - None of the following being available when the entry was written with a data descriptor (defaults to 0):
//!     - CRC
//!     - compressed size
//!     - uncompressed size
//! 
//! See [`stream`] for more information.

// We provide documentation about the differences between the two readers in this module.
// And then usage-level information in the submodules.

pub(crate) mod file;
pub(crate) mod ops;
pub(crate) mod valid;

pub mod seek;
pub mod opts;

#[allow(unused_imports)]
use futures_lite::AsyncSeek;
#[allow(unused_imports)]
use futures_lite::AsyncBufRead;

// Public API
pub use file::ZipFileReader;
