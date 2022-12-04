// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub(crate) mod attribute;
pub mod compression;
pub(crate) mod consts;
pub(crate) mod header;
pub(crate) mod parse;
pub(crate) mod version;

#[cfg(feature = "date")]
pub(crate) mod date;
