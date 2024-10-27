// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use futures_lite::io::{AsyncRead, AsyncWrite, AsyncReadExt};
use crate::error::Result;

pub mod lfh;
pub mod cdr;
pub mod eocdr;

macro_rules! raw {
    ($name:ident { $($field:ident, $type:ty, $read:expr, $write:expr),* }) => {
        use crate::error::Result;
        use futures_lite::io::{AsyncRead, AsyncWrite};

        #[derive(Clone, Copy, Debug)]
        pub struct $name {
            $(pub $field : $type),*
        }

        /// Reads the raw underlying header from the given reader.
        #[tracing::instrument(skip(reader))]
        pub async fn raw_read(mut reader: impl AsyncRead + Unpin) -> Result<$name> {
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

pub(crate) use raw;
