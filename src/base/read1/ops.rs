// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::io::SeekFrom;

use futures_lite::AsyncSeekExt;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncRead;
use futures_lite::AsyncSeek;

#[cfg(feature = "tracing")]
use tracing::{instrument, trace};

use crate::base::read1::opts::ZipOptions;
use crate::base::read1::seek::ZipArchiveInner;
use crate::spec::constructs::EF;
use crate::spec::constructs::EFData;
use crate::spec::constructs::Zip64ExtendedInformation;
use crate::spec::headers1::CDRH;
use crate::spec::constructs::{CDR, LF, EOCDR};
use crate::spec::headers1::EFH;
use crate::spec::headers1::EOCDRH;
use crate::spec::headers1::ExtraFieldHeaderId;
use crate::{error::Result, spec::headers1::{LFH, Signature}};

pub(crate) struct Ops<R> {
    reader: R,
}

impl <R: AsyncRead + Unpin> Ops<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
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

        let lfh = crate::spec::headers1::read::<LFH, R>(&mut self.reader).await?;
        #[cfg(feature = "tracing")]
        trace!("lfh read: {:?}", lfh);

        let mut file_name = vec![0u8; lfh.file_name_length.into()];
        self.reader.read_exact(&mut file_name).await?;
        let extra_fields = Ops::new(&mut self.reader).all_efs(lfh.extra_field_length).await?;

        Ok(LF { lfh, file_name, extra_fields })
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn cdr(&mut self) -> Result<CDR> {
        // We don't assert the signature, as this occurs in the caller's loop.

        let cdrh = crate::spec::headers1::read::<CDRH, R>(&mut self.reader).await?;

        let mut file_name = vec![0u8; cdrh.file_name_length.into()];
        let mut file_comment = vec![0u8; cdrh.file_comment_length.into()];

        self.reader.read_exact(&mut file_name).await?;
        let extra_fields = Ops::new(&mut self.reader).all_efs(cdrh.extra_field_length).await?;
        self.reader.read_exact(&mut file_comment).await?;

        Ok(CDR { cdrh, file_name, extra_fields, file_comment })
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn eocdr(&mut self) -> Result<EOCDR> {
        self.assert_signature(Signature::EOCDRH).await?;

        let eocdrh = crate::spec::headers1::read::<EOCDRH, R>(&mut self.reader).await?;

        let mut file_comment = vec![0u8; eocdrh.file_comm_length.into()];
        self.reader.read_exact(&mut file_comment).await?;

        Ok(EOCDR { eocdrh, file_comment })
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn ef(&mut self) -> Result<EF> {
        let efh = crate::spec::headers1::read::<EFH, R>(&mut self.reader).await?;
        let data;

        match efh.tag {
            ExtraFieldHeaderId::EI64 => {
                let zip64 = crate::spec::headers1::read::<Zip64ExtendedInformation, R>(&mut self.reader).await?;
                data = Some(EFData::Zip64ExtendedInformation(zip64));
            },
            _ => {
                let mut raw = vec![0u8; efh.data_size.into()];
                self.reader.read_exact(&mut raw).await?;
                data = Some(EFData::Unknown(raw));
            }
        };

        Ok(EF { efh, data: data.unwrap() })
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn all_efs(&mut self, size: u16) -> Result<Vec<EF>> {
        let mut take = (&mut self.reader).take(size.into());
        let mut vec = Vec::new();

        // TODO: validation
        // TODO: with_capacity

        loop {
            if take.limit() == 0 {
                break;
            }

            vec.push(Ops::new(&mut take).ef().await?);
        }

        Ok(vec)
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
        let mut inner = ZipArchiveInner::default();

        // Locate EOCDR and seek to it (erroring if not found).
        let eocdr_offset = crate::base::read::io::locator::eocdr(&mut self.reader).await?;
        self.reader.seek(SeekFrom::Start(eocdr_offset - 4)).await?;

        // Read EOCDR and seek to first CDR.
        let eocdr = Ops::new(&mut self.reader).eocdr().await?;
        let pos = eocdr.eocdrh.cent_dir_offset as u64;
        self.reader.seek(SeekFrom::Start(pos)).await?;

        // The callee will not populate metas if load_file_meta is false.
        let (metas, offsets) = SeekOps::new(&mut self.reader).cd(&opts, &eocdr).await?;

        inner.cdr_metas = metas;
        inner.cdr_offsets = offsets;
        inner.options = opts;
    
        Ok(inner)
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn cd(&mut self, options: &ZipOptions, eocdr: &EOCDR) -> Result<(Vec<CDR>, Vec<u64>)> {
        // Enforce max number of files if configured
        if let Some(max) = options.max_num_central_directory_files {
            let actual = eocdr.eocdrh.num_of_entries as u64;

            if actual > max {
                return Err(crate::error::ZipError::CentralDirectoryFilesNumAboveMax(actual, max));
            }
        }

        let mut offset = eocdr.eocdrh.cent_dir_offset as u64;

        // TODO: get size from EOCDR.
        let mut offsets = Vec::new();
        let mut cdrs = Vec::new();

        // TODO: validate size, validate length

        loop {
            let signature = crate::spec::headers1::read::<Signature, R>(&mut self.reader).await?;

            if signature != Signature::CDH {
                break;
            }

            offsets.push(offset);

            let cdr = Ops::new(&mut self.reader).cdr().await?;

            // TODO: don't need to read the whole cdr if we aren't loading metas.
            // Seeking might be less performant than straight reads due to buffering.

            if options.load_file_meta {
                cdrs.push(cdr);
            }

            // TODO: use math instead of seeks for position calc.
            offset = self.reader.seek(SeekFrom::Current(0)).await?;
        }

        // Validate that the number of CDRs read matches the number of entries in the EOCDR
        let num_matched = offsets.len() == eocdr.eocdrh.num_of_entries as usize;
        if options.validate_num_central_directory_files && !num_matched {
            return Err(crate::error::ZipError::InvalidNumCentralDirectoryFiles(
                offsets.len() as u64,
                eocdr.eocdrh.num_of_entries as u64,
            ));
        }

        Ok((cdrs, offsets))
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), level = "trace"))]
    pub async fn file(&mut self, cdr: CDR, opts: &ZipOptions) -> Result<LF> {
        // Seek to file offset and read LF.
        // We read the LF instead of just using the CDR because the extra fields may differ (and so the data offset).
        self.reader.seek(SeekFrom::Start(cdr.cdrh.lh_offset as u64)).await?;
        let mut ops = Ops::new(&mut self.reader);
        let mut lf = ops.lf().await?;

        // Run configured validations on the LF and CDR.
        crate::base::read1::valid::validate(&lf, &cdr, opts)?;

        if lf.lfh.flags.data_descriptor() {
            // Fill in LFH with known-good values from the CDR.
            lf.lfh.compressed_size = cdr.cdrh.compressed_size;
            lf.lfh.uncompressed_size = cdr.cdrh.uncompressed_size;
            lf.lfh.crc = cdr.cdrh.crc;
            
            // TODO: should we set data_descriptor to false?
        }

        Ok(lf)
    }
}
