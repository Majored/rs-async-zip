// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A set of ZIP archive readers which vary based on their seeking vs streaming capabilities.
//! 
//! ## AsyncBufRead requirement
//! All readers must implement [`AsyncBufRead`] (or the tokio equivalent). This stems from the upstream
//! compression crate in use, and we pass this requirement through instead of buffering in-crate. This allows you
//! to control the buffering strategy and avoid double buffering when the underlying reader is already buffered.
//! 
//! This is trivially achieved through [`BufReader`] (or the tokio equivalent). See usage examples for more information.
//! 
//! ## Seeking module
//! The seek reader acts over a single [`AsyncSeek`] reader. See [`seek`] for usage information.
//! 
//! ### Advantages
//! - Can perform out-of-order file reads.
//! - Reads and uses the central directory as the source of truth for file metadata.
//! - Can perform validation of local file headers against the central directory.
//! - Can perform validation of the central directory itself.
//! - Can perform concurrent/parallel file reads when using a factory.
//! 
//! ### Limitations
//! - The underlying reader must implement [`AsyncSeek`] (or the tokio equivalent).
//! - File reads must be sequential (ie. one at a time) unless using a factory.
//! - [`ZipFileReader`] does not support seeking, so nested ZIPs must be opened with [`stream`].
//! 
//! ## Streaming module
//! The stream reader acts over a single non-[`AsyncSeek`] reader. See [`stream`] for usage information.
//! 
//! Support for streaming across the industry is limited. This is because it must read the archive in-order
//! using local file headers which brings its own set of limitations. Consider whether you truly need to
//! stream a ZIP archive. In most cases, saving the stream to disk and using the seek reader is a better option.
//! 
//! ### Advantages
//! - Operating in low-memory environments.
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
//! ## Handling untrusted archives
//! There are many footguns when reading untrusted ZIP archives including malformed archives,
//! insecure file names, nested archives, ZIP differentials, and more. This crate provides a
//! set of options to help mitigate *some* of these risks.
//! 
//! [`ZipOptions::untrusted`] is the recommended starting point, as it enables every validation and
//! bounds everything which we are able to bound using a reasonable default. See [`ZipOptions`] for
//! more information. File names are the one concern which we don't currently handle on your behalf.

// We provide documentation about the differences between the two readers in this module. And then usage-level
// information in the submodules.

pub(crate) mod file;
pub(crate) mod ops;
pub(crate) mod valid;
pub(crate) mod opts;
pub(crate) mod loc;

#[allow(unused_imports)]
use futures_lite::AsyncSeek;
#[allow(unused_imports)]
use futures_lite::AsyncBufRead;
#[allow(unused_imports)]
use futures_lite::io::BufReader;
use crate::error::Result;
use crate::error::ZipError;
#[allow(unused_imports)]
use crate::spec::string::ZipString;

// Public API
pub mod seek;

pub use file::ZipFileReader;
pub use opts::ZipOptions;

pub(crate) fn valid_offset(offset: u64, eor: u64) -> Result<u64> {
    if offset > eor {
        return Err(ZipError::InvalidOffset(offset, eor));
    }

    Ok(offset)
}
