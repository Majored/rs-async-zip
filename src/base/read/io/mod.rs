// Copyright (c) 2022 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub(crate) mod combined_record;
pub(crate) mod compressed;
pub(crate) mod entry;
pub(crate) mod hashed;
pub(crate) mod locator;
pub(crate) mod owned;

use std::{
    future::Future,
    io::ErrorKind,
    pin::Pin,
    task::{ready, Context, Poll},
};

pub use combined_record::CombinedCentralDirectoryRecord;
use futures_lite::io::AsyncBufRead;
use pin_project::pin_project;

use crate::{
    spec::consts::{DATA_DESCRIPTOR_LENGTH, DATA_DESCRIPTOR_SIGNATURE, SIGNATURE_LENGTH},
    string::{StringEncoding, ZipString},
};
use futures_lite::io::{AsyncRead, AsyncReadExt};

/// Read and return a dynamic length string from a reader which impls AsyncRead.
pub(crate) async fn read_string<R>(reader: R, length: usize, encoding: StringEncoding) -> std::io::Result<ZipString>
where
    R: AsyncRead + Unpin,
{
    Ok(ZipString::new(read_bytes(reader, length).await?, encoding))
}

/// Read and return a dynamic length vector of bytes from a reader which impls AsyncRead.
pub(crate) async fn read_bytes<R>(reader: R, length: usize) -> std::io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = Vec::with_capacity(length);
    reader.take(length as u64).read_to_end(&mut buffer).await?;

    Ok(buffer)
}

#[pin_project]
pub(crate) struct ConsumeDataDescriptor<'a, R>(#[pin] pub(crate) &'a mut R);

impl<R> Future for ConsumeDataDescriptor<'_, R>
where
    R: AsyncBufRead + Unpin,
{
    type Output = std::io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut project = self.project();

        let data = poll_result_ok!(ready!(project.0.as_mut().poll_fill_buf(cx)));
        let signature = data.get(0..4).ok_or(ErrorKind::UnexpectedEof)?;
        let mut consumed = DATA_DESCRIPTOR_LENGTH;

        if signature == DATA_DESCRIPTOR_SIGNATURE.to_le_bytes() {
            consumed += SIGNATURE_LENGTH;
        }
        if consumed > data.len() {
            return Poll::Ready(Err(ErrorKind::UnexpectedEof.into()));
        }

        project.0.as_mut().consume(consumed);
        Poll::Ready(Ok(()))
    }
}

/// A macro that returns the inner value of an Ok or early-returns in the case of an Err.
///
/// This is almost identical to the ? operator but handles the situation when a Result is used in combination with
/// Poll (eg. tokio's IO traits such as AsyncRead).
macro_rules! poll_result_ok {
    ($poll:expr) => {
        match $poll {
            Ok(inner) => inner,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

use poll_result_ok;
