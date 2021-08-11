use crate::error::ZIPError;
use async_compression::tokio::bufread::DeflateDecoder;
use std::marker::{Send, Unpin};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt};

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;
use tokio::io::Take;

type Reader = dyn AsyncBufRead + Unpin + Send;

pub struct ZIPStreamReader<'a> {
    reader: &'a mut Reader,
}

pub struct ZIPStreamFile<'a> {
    file_name: String,
    size: u32,
    reader: DeflateDecoder<Take<&'a mut Reader>>,
}

impl<'a> AsyncRead for ZIPStreamFile<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl<'a> ZIPStreamFile<'a> {
    /// Returns a reference to the file's name.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the file's size in bytes.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Consumes all bytes within this file.
    ///
    /// Any file's contents will need to be fully read before a call to ZIPStreamReader::next_entry() is made so that
    /// the inner reader is already positioned at the start of the local file header deliminator. If you don't want to
    /// fully read the file content's yourself, this method can be called to consume the bytes for you before dropping.
    pub async fn consume(&mut self) -> Result<(), ZIPError> {
        let mut buffer = vec![0; 8192];

        loop {
            match self.reader.read(&mut buffer).await {
                Ok(read) => {
                    if read == 0 {
                        break;
                    }
                }
                Err(_) => return Err(ZIPError::ReadFailed),
            };
        }

        Ok(())
    }
}

impl<'a> ZIPStreamReader<'a> {
    pub fn from_reader(reader: &mut Reader) -> Result<ZIPStreamReader<'_>, ZIPError> {
        Ok(ZIPStreamReader { reader })
    }

    pub async fn next_entry(&mut self) -> Result<ZIPStreamFile<'_>, ZIPError> {
        if read_u32(self.reader).await? != crate::consts::LFHD {
            return Err(ZIPError::LocalFileHeaderError);
        }

        let version = read_u16(self.reader).await?;
        let flags = read_u16(self.reader).await?;
        let compression = read_u16(self.reader).await?;
        let mod_time = read_u16(self.reader).await?;
        let mod_date = read_u16(self.reader).await?;
        let crc = read_u32(self.reader).await?;
        let compressed_size = read_u32(self.reader).await?;
        let uncompressed_size = read_u32(self.reader).await?;
        let file_name_length = read_u16(self.reader).await?;
        let exta_field_length = read_u16(self.reader).await?;

        let file_name = read_string(self.reader, file_name_length).await?;
        let exta_field = read_string(self.reader, exta_field_length).await?;

        Ok(ZIPStreamFile {
            file_name,
            size: uncompressed_size,
            reader: DeflateDecoder::new(self.reader.take(compressed_size.into())),
        })
    }
}

async fn read_string(reader: &mut Reader, length: u16) -> Result<String, ZIPError> {
    let mut buffer = String::with_capacity(length as usize);
    reader
        .take(length as u64)
        .read_to_string(&mut buffer)
        .await
        .map_err(|_| ZIPError::ReadFailed)?;
    Ok(buffer)
}

async fn read_u32(reader: &mut Reader) -> Result<u32, ZIPError> {
    Ok(reader
        .read_u32_le()
        .await
        .map_err(|_| ZIPError::ReadFailed)?)
}

async fn read_u16(reader: &mut Reader) -> Result<u16, ZIPError> {
    Ok(reader
        .read_u16_le()
        .await
        .map_err(|_| ZIPError::ReadFailed)?)
}
