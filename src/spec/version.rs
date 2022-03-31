// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::compression::Compression;
use crate::write::EntryOptions;

pub(crate) const SPEC_VERSION_MADE_BY: u16 = 63;

// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#443
pub fn as_needed_to_extract(options: &EntryOptions) -> u16 {
    let mut version = match options.compression {
        Compression::Deflate => 20,
        Compression::Bz => 46,
        Compression::Lzma => 63,
        _ => 10,
    };

    if options.filename.ends_with('/') {
        version = std::cmp::max(version, 20);
    }

    version
}

// https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md#442
pub fn as_made_by() -> u16 {
    // Default to UNIX mapping for the moment.
    3 << 8 | SPEC_VERSION_MADE_BY
}
