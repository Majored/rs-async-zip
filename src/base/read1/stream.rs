// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::ZipError;
use crate::spec::headers1::{Compression, Signature};

use futures_lite::{AsyncBufRead};

use crate::{base::read1::{file::ZipFileReader, ops::{Ops}, opts::ZipOptions}, error::Result, spec::constructs::{LF}};

/// A ZIP archive reader which acts over a non-seekable source.
pub struct ZipArchiveReader<R> {
    state: Option<State<R>>,
    options: ZipOptions,
    seen: Vec<LF>,
}

impl<R: AsyncBufRead + Unpin> ZipArchiveReader<R> {
    pub fn new(reader: R) -> Self {
        Self::new_with_options(reader, ZipOptions::default())
    }

    pub fn new_with_options(reader: R, options: ZipOptions) -> Self {
        Self { state: Some(State::Ready(reader)), options, seen: Vec::new() }
    }

    /// Drains whatever is left of the current entry (if any) and reclaims the reader.
    async fn finish_current(&mut self) -> Result<()> {
        let state = self.state.take().expect("ZipArchiveReader state invariant violated");

        let reader = match state {
            State::Ready(reader) => reader,
            State::Reading(mut entry) => {
                entry.skip().await?;
                entry.into_inner()
            }
        };

        self.state = Some(State::Ready(reader));
        Ok(())
    }

    /// If you get an error, you may continue to call `next()` to try to read the next file.
    pub async fn next(&mut self) -> Result<Option<&mut ZipFileReader<R>>> {
        self.finish_current().await?;

        let mut reader = match self.state.take().expect("ZipArchiveReader state invariant violated") {
            State::Ready(reader) => reader,
            State::Reading(_) => unreachable!("finish_current always leaves state as Ready"),
        };

        // We shouldn't ever get to EOF before one of these signatures.
        let signature = crate::spec::headers1::read::<Signature, R>(&mut reader).await?;

        let lf = match signature {
            Signature::LFH => Ops::new(&mut reader, &self.options).lf(false).await?,
            Signature::CDRH | Signature::EOCDRH | Signature::EOCDR64H => {
                self.state = Some(State::Ready(reader));
                self.validate_cdr().await?;
                return Ok(None);
            }
            signature => {
                self.state = Some(State::Ready(reader));
                return Err(ZipError::UnexpectedHeaderError(signature.into(), Signature::LFH.into()));
            }
        };

        if lf.lfh.gpf.data_descriptor() {
            // TODO: add support back
            self.state = Some(State::Ready(reader));
            return Err(ZipError::FeatureNotSupported("stream reading data descriptors"));
        }
        if lf.lfh.gpf.data_descriptor() && lf.lfh.compression == Compression::Stored {
            self.state = Some(State::Ready(reader));
            return Err(ZipError::FeatureNotSupported("stream reading data descriptors with stored compression"));
        }

        self.seen.push(lf.clone());
        self.state = Some(State::Reading(ZipFileReader::new(reader, lf, None, self.options.clone())?));

        match &mut self.state {
            Some(State::Reading(entry)) => Ok(Some(entry)),
            _ => unreachable!(),
        }
    }

    async fn validate_cdr(&mut self) -> Result<()> {
        todo!()
    }
}

enum State<R> {
    Ready(R),
    Reading(ZipFileReader<R>),
}
