// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A ZIP reader which acts over a non-seekable source.

use crate::base::read::stream::Reading;
use crate::base::read::stream::Ready;
use crate::entry::ZipEntry;
use crate::error::Result;
use crate::base::read::io::entry::ZipEntryReader;
use crate::base::read::stream::ZipFileReader as BaseZipFileReader;

use futures_util::io::Take;
use tokio::io::AsyncRead;
use tokio_util::compat::Compat;
use tokio_util::compat::TokioAsyncReadCompatExt;

// TODO: Remove exposed Compat wrappers from public API.

/// A ZIP reader which acts over a non-seekable source.
///
/// See the [module-level docs](.) for more information.
#[derive(Clone)]
pub struct ZipFileReader<S>(BaseZipFileReader<S>);

impl<'a, R> ZipFileReader<Ready<Compat<R>>>
where
    R: AsyncRead + Unpin + 'a,
{
    /// Constructs a new ZIP reader from a non-seekable source.
    pub fn new(reader: R) -> Self {
        ZipFileReader(BaseZipFileReader::new(reader.compat()))
    }

    /// Opens the next entry for reading if the central directory hasn’t yet been reached.
    pub async fn next_entry(self) -> Result<Option<ZipFileReader<Reading<'a, Take<Compat<R>>>>>> {
        let entry_option = self.0.next_entry().await?;

        if let Some(inner) = entry_option {
            return Ok(Some(ZipFileReader(inner)));
        }

        return Ok(None);
    }

    /// Consumes the `ZipFileReader` returning the original `reader`
    pub async fn into_inner(self) -> Compat<R> {
        self.0.into_inner().await
    }
}

impl<'a, R> ZipFileReader<Reading<'a, Take<Compat<R>>>>
where
    R: AsyncRead + Unpin,
{
    /// Returns the current entry's data.
    pub fn entry(&self) -> &ZipEntry {
        self.0.entry()
    }

    /// Returns a mutable reference to the inner entry reader.
    pub fn reader(&mut self) -> &mut ZipEntryReader<'a, Take<Compat<R>>> {
        self.0.reader()
    }

    /// Converts the reader back into the Ready state if EOF has been reached.
    pub async fn done(self) -> Result<ZipFileReader<Ready<Compat<R>>>> {
        Ok(ZipFileReader(self.0.done().await?))
    }

    /// Reads until EOF and converts the reader back into the Ready state.
    pub async fn skip(self) -> Result<ZipFileReader<Ready<Compat<R>>>> {
        Ok(ZipFileReader(self.0.skip().await?))
    }
}
