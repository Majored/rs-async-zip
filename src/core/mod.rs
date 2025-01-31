// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! Core functionality for reading and writing ZIP files.
//! 
//! Implementation notes:
//! - any AsyncBufRead + AsyncSeek impl discards the buffer on seek, such as tokio.
//! * assumes the ZIP file is the only content in the stream (ie. the start of the ZIP file is at the start
// * of the stream, and the end of the ZIP file is at the end of the stream). This assumption is therefor
// * carried forward. 
// * assumes offsets located in CDR are relative to the start of the stream

pub mod ops;
pub mod wrap;
pub mod raw;
pub mod cd;

pub const SIGNATURE_LENGTH: usize = size_of::<u32>();