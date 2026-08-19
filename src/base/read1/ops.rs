// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::SeekFrom;

use futures_lite::AsyncSeekExt;
use futures_lite::AsyncRead;
use futures_lite::AsyncSeek;

#[cfg(feature = "tracing")]
use tracing::{instrument, trace};

use crate::base::read1::opts::ZipOptions;
use crate::base::read1::seek::ZipArchiveInner;
use crate::error::ZipError;
use crate::spec::constructs::CombinedEOCDR;
use crate::spec::headers1::CDRH;
use crate::spec::constructs::{CDR, LF, EOCDR};
use crate::spec::headers1::EOCDR64H;
use crate::spec::headers1::EOCDRH;
use crate::spec::KnownSize;
use crate::spec::headers1::EOCDL64H;
use crate::{error::Result, spec::headers1::{LFH, Signature}};

pub(crate) struct Ops<'o, R> {
    options: &'o ZipOptions,
    reader: R,
}

impl<'o, R: AsyncRead + Unpin> Ops<'o, R> {
    pub fn new(reader: R, options: &'o ZipOptions) -> Self {
        Self { reader, options }
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn assert_signature(&mut self, expected: Signature) -> Result<()> {
        let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await?;
        #[cfg(feature = "tracing")]
        trace!("read signature: {:02X?}", signature);

        if signature != expected {
            return Err(crate::error::ZipError::UnexpectedHeaderError(signature.into(), expected.into()));
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn lf(&mut self) -> Result<LF> {
        self.assert_signature(Signature::LFH).await?;

        let options = self.options;
        let lf = crate::spec::headers1::read_record::<LFH, LF, R>(&mut self.reader, |lfh| {
            crate::base::read1::valid::validate_extra_field_size(lfh.extra_field_length, options)?;
            Ok(usize::from(lfh.file_name_length) + usize::from(lfh.extra_field_length))
        })
        .await?;

        crate::base::read1::valid::validate_extra_field_num(&lf.efs, options)?;

        Ok(lf)
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn cdr(&mut self) -> Result<CDR> {
        // We don't assert the signature, as this occurs in the caller's loop.

        let options = self.options;
        let cdr = crate::spec::headers1::read_record::<CDRH, CDR, R>(&mut self.reader, |cdrh| {
            crate::base::read1::valid::validate_extra_field_size(cdrh.extra_field_length, options)?;
            Ok(usize::from(cdrh.file_name_length)
                + usize::from(cdrh.extra_field_length)
                + usize::from(cdrh.file_comment_length))
        })
        .await?;

        crate::base::read1::valid::validate_extra_field_num(&cdr.efs, options)?;

        Ok(cdr)
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn eocdr(&mut self) -> Result<EOCDR> {
        self.assert_signature(Signature::EOCDRH).await?;

        crate::spec::headers1::read_record::<EOCDRH, EOCDR, R>(&mut self.reader, |eocdrh| {
            Ok(usize::from(eocdrh.file_comm_length))
        })
        .await
    }
}

pub(crate) struct SeekOps<R> {
    reader: R,
}

impl<R: AsyncRead + AsyncSeek + Unpin> SeekOps<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn open(&mut self, opts: ZipOptions) -> Result<ZipArchiveInner> {
        let eocdr_offset = crate::base::read::io::locator::eocdr(&mut self.reader).await?;
        self.reader.seek(SeekFrom::Start(eocdr_offset - 4)).await?;

        let eocdr = Ops::new(&mut self.reader, &opts).eocdr().await?;
        let mut combined = CombinedEOCDR { eocdr, eocdr64: None, eocdl64: None };

        if let Some((locator, record)) = SeekOps::new(&mut self.reader).zip64(eocdr_offset, &opts).await? {
            combined.eocdr64 = Some(record);
            combined.eocdl64 = Some(locator);
        }

        self.reader.seek(SeekFrom::Start(combined.cd_offset())).await?;
        crate::base::read1::valid::validate_archive(&combined, &opts)?;
        let (loaded_cdrs, offsets) = SeekOps::new(&mut self.reader).cd(&opts, &combined).await?;

        let inner = ZipArchiveInner {
            loaded_cdrs,
            cdr_offsets: offsets,
            options: opts,
            combined_eocdr: combined,
        };
    
        Ok(inner)
    }

    pub async fn zip64(&mut self, eocdr_offset: u64, opts: &ZipOptions) -> Result<Option<(EOCDL64H, EOCDR64H)>> {
        // TODO: wrapping
        let zip64_locator_pos = eocdr_offset - (Signature::SIZE as u64 * 2) - EOCDL64H::SIZE as u64;
        self.reader.seek(SeekFrom::Start(zip64_locator_pos)).await?;
        let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await;

        if let Err(ZipError::BinaryParseError(_)) = signature {
            // Invalid signature, which means there is no ZIP64 locator.
            // TODO: Signature being a discriminant enum makes this a bit awkward.
            return Ok(None);
        }
        if !matches!(signature, Ok(Signature::EOCDL64H)) {
            // TODO: We could reasonable assume that if we hit a known signature,
            //       the archive is malformed, not just that it's not ZIP64.
            return Ok(None);
        }

        let eocdl64h = crate::spec::headers1::read::<EOCDL64H, R>(&mut self.reader).await?;
        self.reader.seek(SeekFrom::Start(eocdl64h.relative_offset)).await?;
        Ops::new(&mut self.reader, opts).assert_signature(Signature::EOCDR64H).await?;

        let eocdr64h = crate::spec::headers1::read::<EOCDR64H, R>(&mut self.reader).await?;

        Ok(Some((eocdl64h, eocdr64h)))
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn cd(&mut self, options: &ZipOptions, eocdr: &CombinedEOCDR) -> Result<(Vec<CDR>, Vec<u64>)> {
        let cdrs_capacity = eocdr.num_entries().min(options.max_num_cd_files_load);
        let mut offset = eocdr.cd_offset();

        // We load all the offsets regardless as we need to seek to the CDRs at a minimum.
        let mut offsets = Vec::with_capacity(eocdr.num_entries() as usize);
        let mut cdrs = Vec::with_capacity(cdrs_capacity as usize);

        // TODO: validate size, validate length, enforce limits

        loop {
            let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await?;

            if signature != Signature::CDH {
                break;
            }

            offsets.push(offset);

            let cdr = Ops::new(&mut self.reader, options).cdr().await?;

            // TODO: don't need to read the whole cdr if we aren't loading cdrs.
            // Seeking might be less performant than straight reads due to buffering.

            // A non-zero capacity means we are loading CDRs into memory, and empty archives shouldn't get here.
            if cdrs.capacity() != 0 {
                cdrs.push(cdr);
            }

            // TODO: use math instead of seeks for position calc.
            offset = self.reader.seek(SeekFrom::Current(0)).await?;
        }

        // Validate that the number of CDRs read matches the number of entries in the EOCDR
        let num_matched = offsets.len() == eocdr.num_entries() as usize;
        if options.validate_num_central_directory_files && !num_matched {
            return Err(crate::error::ZipError::NumFilesMismatch(offsets.len() as u64, eocdr.num_entries()));
        }

        Ok((cdrs, offsets))
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn file(&mut self, cdr: CDR, opts: &ZipOptions) -> Result<LF> {
        // We read the LF instead of just using the CDR because the extra fields may differ (and so the data offset).
        self.reader.seek(SeekFrom::Start(cdr.lfh_offset() as u64)).await?;
        let mut ops = Ops::new(&mut self.reader, opts);
        let mut lf = ops.lf().await?;

        crate::base::read1::valid::validate_file(&lf, &cdr, opts)?;

        if lf.lfh.flags.data_descriptor() {
            // We know the values from the CDR are 'good', because they were written after the file finished writing.
            lf.lfh.compressed_size = cdr.cdrh.compressed_size;
            lf.lfh.uncompressed_size = cdr.cdrh.uncompressed_size;
            lf.lfh.crc = cdr.cdrh.crc;
            
            // TODO: should we set data_descriptor to false?
            // TODO: copy over Zip64EI
        }

        Ok(lf)
    }
}
