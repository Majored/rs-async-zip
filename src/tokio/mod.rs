// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A mirrored implementation with [`tokio`]-specific IO types and features.

#[cfg(doc)]
use tokio;

pub mod read;

pub mod write {
    use tokio_util::compat::Compat;

    pub type ZipFileWriter<W> = crate::base::write::ZipFileWriter<Compat<W>>;
    pub type EntryStreamWriter<'a, W> = crate::base::write::EntryStreamWriter<'a, Compat<W>>;
}
