// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#![allow(dead_code)]

use crate::Compression;

pub struct ZipEntryOptions {
    pub(crate) filename: String,
    pub(crate) compression: Compression,
}

impl ZipEntryOptions {
    pub fn new(filename: String, compression: Compression) -> Self {
        ZipEntryOptions { filename, compression }
    }
}
