// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// piggybacks off of seek implementation.
// replaces both mem & tokio::fs imppls in previous versions of this crate.

// index -> offset
// 
// if meta loaded:
// index -> 

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_lite::{AsyncRead, io::Cursor};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, SemaphorePermit};

use crate::base::read1::seek::SeekInner;

// owned
// vs
// borrowed

// provide name lookup
//
struct ZipArchiveReader<R: AsyncRead, G: AsyncFnMut() -> R> {
    generator: G,
    inner: SeekInner,
}

impl <R: AsyncRead, G: AsyncFnMut() -> R> ZipArchiveReader<R, G> {
    pub fn new(generator: G) -> Self {
        Self { generator, inner: SeekInner::default() }
    }

    pub fn with_options(generator: G, options: Options) -> Self {
        Self { generator, inner: SeekInner::with_options(options) }
    }
}

pub async fn test() {
    let mut sem = Arc::new(Semaphore::new(1));

    let mut reader = ZipArchiveReader::new(async move || { 
        let sem1 = sem.clone().acquire_owned().await.unwrap();
        SemaphoreGuarded { semaphore: sem1, reader: Cursor::new(vec![0u8; 10]) }
    }) ;
}

struct SemaphoreGuarded<R> {
    semaphore: OwnedSemaphorePermit,
    reader: R,
}

impl<R: AsyncRead + Unpin> AsyncRead for SemaphoreGuarded<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}