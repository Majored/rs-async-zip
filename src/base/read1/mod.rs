// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of ZIP archive readers which vary based on their seeking vs streaming capabilities.
//! 
//! ## Seek
//! The seek reader acts over a single [`AsyncSeek`] reader. It supports out-of-order reads.
//! This re-uses the same reader for all reads, meaning reads must be sequential.
//! 
//! ### Advantages
//! - Can perform validation of local file headers against the central directory.
//! 
//! ### Limitations
//! - 
//! 
//! See [`seek`] for more information.
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

pub mod seek;
pub mod ops;
pub mod file;
pub mod valid;
pub mod opts;

#[allow(unused_imports)]
use futures_lite::AsyncSeek;
