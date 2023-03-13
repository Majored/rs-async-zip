// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A mirrored implementation with [`tokio`]-specific IO types and features.

#[cfg(doc)]
use tokio;

pub mod read;
pub mod write;
