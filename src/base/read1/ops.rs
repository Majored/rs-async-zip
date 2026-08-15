// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::SeekFrom;

use futures_lite::AsyncSeekExt;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncRead;
use futures_lite::AsyncSeek;

use crate::base::read1::opts::Options;
use crate::base::read1::seek::ZipArchiveInner;
use crate::spec::headers1::CDRH;
use crate::spec::headers1::EOCDR;
use crate::spec::headers1::EOCDRH;
use crate::{error::Result, spec::headers1::{CDR, LF, LFH, Signature}};

pub(crate) struct Ops<R> {
    reader: R,
}

impl <R: AsyncRead + Unpin> Ops<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub async fn assert_signature(&mut self, expected: Signature) -> Result<()> {
        let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await?;

        if signature != expected {
            return Err(crate::error::ZipError::UnexpectedHeaderError(signature.into(), expected.into()));
        }

        Ok(())
    }

    pub async fn lf(&mut self) -> Result<LF> {
        self.assert_signature(Signature::LFH).await?;
        let lfh = crate::spec::headers1::read::<LFH, R>(&mut self.reader).await?;

        let mut file_name = vec![0u8; lfh.file_name_length.into()]; 
        let mut extra_field = vec![0u8; lfh.extra_field_length.into()];

        self.reader.read_exact(&mut file_name).await?;
        self.reader.read_exact(&mut extra_field).await?;

        Ok(LF { lfh, file_name, extra_field })
    }

    pub async fn cdr(&mut self) -> Result<CDR> {
        // We don't assert the signature, as this occurs in the caller's loop.

        let cdrh = crate::spec::headers1::read::<CDRH, R>(&mut self.reader).await?;

        let mut file_name = vec![0u8; cdrh.file_name_length.into()];
        let mut extra_field = vec![0u8; cdrh.extra_field_length.into()];
        let mut file_comment = vec![0u8; cdrh.file_comment_length.into()];

        self.reader.read_exact(&mut file_name).await?;
        self.reader.read_exact(&mut extra_field).await?;
        self.reader.read_exact(&mut file_comment).await?;

        Ok(CDR { cdrh, file_name, extra_field, file_comment })
    }

    pub async fn eocdr(&mut self) -> Result<EOCDR> {
        self.assert_signature(Signature::EOCDRH).await?;

        let eocdrh = crate::spec::headers1::read::<EOCDRH, R>(&mut self.reader).await?;
        let mut file_comment = vec![0u8; eocdrh.file_comm_length.into()];

        self.reader.read_exact(&mut file_comment).await?;

        Ok(EOCDR { eocdrh, file_comment })
    }
}

pub(crate) struct SeekOps<R> {
    reader: R,
}

impl<R: AsyncRead + AsyncSeek + Unpin> SeekOps<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub async fn cd_offsets(&mut self) -> Result<Vec<u64>> {
        todo!()
    }

    pub async fn open(&mut self, opts: Options) -> Result<ZipArchiveInner> {
        let mut inner = ZipArchiveInner::default();

        // Locate EOCDR and seek to it (erroring if not found).
        let eocdr_offset = crate::base::read::io::locator::eocdr(&mut self.reader).await?;
        self.reader.seek(SeekFrom::Start(eocdr_offset - 4)).await?;

        // Read EOCDR and seek to first CDR.
        let eocdr = Ops::new(&mut self.reader).eocdr().await?;
        let pos = eocdr.eocdrh.cent_dir_offset as u64;
        self.reader.seek(SeekFrom::Start(pos)).await?;

        // The callee will not populate metas if load_file_meta is false.
        let (metas, offsets) = SeekOps::new(&mut self.reader).cd(&inner.options, pos).await?;

        inner.cdr_metas = metas;
        inner.cdr_offsets = offsets;
    
        Ok(inner)
    }

    pub async fn cd(&mut self, options: &Options, mut offset: u64) -> Result<(Vec<CDR>, Vec<u64>)> {
        // TODO: get size from EOCDR.
        let mut offsets = Vec::new();
        let mut cdrs = Vec::new();

        // TODO: validate size, validate length

        loop {
            offsets.push(offset);
            let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await?;

            if signature != Signature::CDH {
                break;
            }

            let cdr = Ops::new(&mut self.reader).cdr().await?;

            // TODO: don't need to read the whole cdr if we aren't loading metas.
            // Seeking might be less performant than straight reads due to buffering.

            if options.load_file_meta {
                cdrs.push(cdr);
            }

            // TODO: use math instead of seeks for position calc.
            offset = self.reader.seek(SeekFrom::Current(0)).await?;
        }

        Ok((cdrs, offsets))
    }

    pub async fn file(&mut self, cdr: CDR, opts: &Options) -> Result<LF> {
        // Seek to file offset and read LF.
        // We read the LF instead of just using the CDR because the extra fields may differ (and so the data offset).
        self.reader.seek(SeekFrom::Start(cdr.cdrh.lh_offset as u64)).await?;
        let mut ops = Ops::new(&mut self.reader);
        let mut lf = ops.lf().await?;

        // Run configured validations on the LF and CDR.
        crate::base::read1::valid::validate(&lf, &cdr, opts)?;

        // TODO: data descriptor.
        if false {
            // Fill in LFH with known-good values from the CDR.
            lf.lfh.compressed_size = cdr.cdrh.compressed_size;
            lf.lfh.uncompressed_size = cdr.cdrh.uncompressed_size;
            lf.lfh.crc = cdr.cdrh.crc;
        }

        Ok(lf)
    }
}
