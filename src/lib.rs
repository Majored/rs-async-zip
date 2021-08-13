// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! # async_zip
//! 
//! An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.
//! 
//! ## Features
//! - Asynchronous design powered by `tokio`.
//! - Support for Stored, Deflate, Bzip2, LZMA, zstd, and xz compression methods.
//! - Aims for resonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.

pub(crate) mod delim;
pub(crate) mod header;
pub mod error;
pub mod stream;