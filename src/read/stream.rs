// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::Result;
use crate::error::ZipError;
use crate::spec::header::LocalFileHeader;
use crate::utils::assert_signature;
use crate::{AttributeCompatibility, Compression, ZipDateTime, ZipEntry};
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncRead, BufReader};

use super::seek::ZipEntryReader;

struct Ready<R>(R);
struct Reading<'a, R>(ZipEntryReader<'a, R>, ZipEntry);

/// A ZIP reader which acts over a non-seekable source.
///
/// See the [module-level docs](.) for more information.
#[derive(Clone)]
pub struct ZipFileReader<S>(S);

impl<'a, R> ZipFileReader<Ready<R>>
where
    R: AsyncRead + Unpin + 'a,
{
    /// Constructs a new ZIP reader from a non-seekable source.
    pub fn new(reader: R) -> Self {
        Self(Ready(reader))
    }

    /// Opens the next entry for reading if the central directory hasn’t yet been reached.
    pub async fn next_entry(mut self) -> Result<Option<ZipFileReader<Reading<'a, R>>>> {
        assert_signature(&mut self.0 .0, crate::spec::consts::LFH_SIGNATURE).await?;

        let header = LocalFileHeader::from_reader(&mut self.0 .0).await?;
        let filename = crate::read::io::read_string(&mut self.0 .0, header.file_name_length.into()).await?;
        let compression = Compression::try_from(header.compression)?;
        let extra_field = crate::read::io::read_bytes(&mut self.0 .0, header.extra_field_length.into()).await?;

        let entry = ZipEntry {
            filename,
            compression,
            #[cfg(any(feature = "deflate", feature = "bzip2", feature = "zstd", feature = "lzma", feature = "xz"))]
            compression_level: async_compression::Level::Default,
            attribute_compatibility: AttributeCompatibility::Unix,
            /// FIXME: Default to Unix for the moment
            crc32: header.crc,
            uncompressed_size: header.uncompressed_size,
            compressed_size: header.compressed_size,
            last_modification_date: ZipDateTime { date: header.mod_date, time: header.mod_time },
            internal_file_attribute: 0,
            external_file_attribute: 0,
            extra_field,
            comment: String::new(),
        };

        let reader = BufReader::new(self.0 .0);
        Ok(Some(ZipFileReader(Reading(
            ZipEntryReader::new_with_owned(reader, compression, entry.uncompressed_size.into()),
            entry,
        ))))
    }
}

impl<'a, R> ZipFileReader<Reading<'a, R>>
where
    R: AsyncRead + Unpin,
{
    /// Returns the current entry's data.
    pub fn entry(&self) -> &ZipEntry {
        &self.0 .1
    }

    /// Returns a mutable reference to the inner entry reader.
    pub fn reader(&mut self) -> &mut ZipEntryReader<'a, R> {
        &mut self.0 .0
    }

    /// Converts the reader back into the Ready state if EOF has been reached.
    pub async fn done(mut self) -> Result<ZipFileReader<Ready<R>>> {
        if self.0 .0.read(&mut [0; 1]).await? != 0 {
            return Err(ZipError::CRC32CheckError); // CHANGE
        }

        // FIXME: Need to impl into into_inner() for ZipEntryReader.
        todo!();
    }

    /// Reads until EOF and converts the reader back into the Ready state.
    pub async fn skip(mut self) -> Result<ZipFileReader<Ready<R>>> {
        while self.0 .0.read(&mut [0; 2048]).await? != 0 {}

        // FIXME: Need to impl into into_inner() for ZipEntryReader.
        todo!();
    }
}
