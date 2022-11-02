// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use async_compression::Level;

// Developer note: This is a copy of async_compression::Level to hide
// implementation details and allow easier updates.

/// Level of compression data should be compressed with.
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// Fastest quality of compression, usually produces bigger size.
    Fastest,
    /// Best quality of compression, usually produces the smallest size.
    Best,
    /// Default quality of compression defined by the selected compression algorithm.
    Default,
    /// Precise quality based on the underlying compression algorithms'
    /// qualities. The interpretation of this depends on the algorithm chosen
    /// and the specific implementation backing it.
    /// Qualities are implicitly clamped to the algorithm's maximum.
    Precise(u32),
}

impl CompressionLevel {
    pub(crate) fn into_level(self) -> Level {
        match self {
            CompressionLevel::Fastest => Level::Fastest,
            CompressionLevel::Best => Level::Best,
            CompressionLevel::Default => Level::Default,
            CompressionLevel::Precise(n) => Level::Precise(n),
        }
    }
}
