// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#![allow(dead_code)]

/// Local file header delimiter.
pub const LFHD: u32 = 0x04034b50;

/// Central directory file header delimiter.
pub const CDFHD: u32 = 0x02014b50;

/// Data descriptor delimiter.
pub const DDD: u32 = 0x08074b50;

/// End of central directory delimiter.
pub const EOCDD: u32 = 0x06054b50;
