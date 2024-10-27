// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub mod cd;
pub mod cdr;
pub mod dd;
pub mod edf;
pub mod eocdr;
pub mod lfh;
pub mod zip64;

/// The length in bytes of a signature. All signatures are u32s, so 4 bytes long.
pub const SIGNATURE_LENGTH: usize = 4;

macro_rules! raw {
    ($name:ident { $($field:ident, $type:ty, $read:expr, $write:expr),* }) => {
        use crate::error::Result;
        use futures_lite::io::{AsyncBufRead, AsyncWrite};

        #[derive(Clone, Copy, Debug)]
        pub struct $name {
            $(pub $field : $type),*
        }

        /// Reads the raw underlying header from the given reader.
        #[tracing::instrument(skip(reader))]
        pub async fn raw_read(mut reader: impl AsyncBufRead + Unpin) -> Result<$name> {
            Ok($name {
                $($field : $read(&mut reader).await? ),*
            })
        }

        /// Writes the raw underlying header to the given writer.
        #[tracing::instrument(skip(writer, raw))]
        pub async fn raw_write(mut writer: impl AsyncWrite + Unpin, raw: &$name) -> Result<()> {
            $($write(&mut writer, raw.$field).await?;)*
            Ok(())
        }
    }
}

macro_rules! raw_deref {
    ($from:ident, $to:ident) => {
        use std::ops::{Deref, DerefMut};

        impl Deref for $from {
            type Target = $to;

            fn deref(&self) -> &Self::Target {
                &self.raw
            }
        }

        impl DerefMut for $from {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.raw
            }
        }
    };
}

pub(crate) use raw;
pub(crate) use raw_deref;
