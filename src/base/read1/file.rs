// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_lite::AsyncReadExt;
use futures_lite::AsyncRead;
use futures_lite::io::Take;

use crate::error::Result;
use crate::spec::headers1::LF;

pub struct ZipFileReader<R> {
    reader: Take<R>,
    lf: LF,
}

impl<R: AsyncRead + Unpin> ZipFileReader<R> {
    pub fn new(reader: R, lf: LF) -> Result<Self> {
        if lf.lfh.compression != 0 {
            // TODO: only supports Stored currently.
            return Err(crate::error::ZipError::CompressionNotSupported(lf.lfh.compression));
        }

        Ok(Self { reader: reader.take(lf.lfh.compressed_size as u64), lf })
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ZipFileReader<R> {
    fn poll_read(mut self: Pin<&mut Self>,cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}
