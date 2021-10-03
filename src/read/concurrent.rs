// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::error::Result;
use crate::read::{ZipEntry, ZipEntryReader};

use tokio::fs::File;
use tokio::io::AsyncSeekExt;

use std::io::SeekFrom;

pub struct ZipFileReader<'a> {
    pub(crate) filename: &'a str,
    pub(crate) entries: Vec<(u32, ZipEntry)>,
}

impl<'a> ZipFileReader<'a> {
    pub async fn new(filename: &'a str) -> Result<ZipFileReader<'a>> {
        let mut fs_file = File::open(filename).await?;
        let entries = crate::read::seek::read_cd(&mut fs_file).await?;

        Ok(ZipFileReader { filename, entries })
    }

    pub async fn get(&self, index: usize) -> Result<Option<ZipEntryReader>> {
        let (offset, entry) = match self.entries.get(index) {
            Some(value) => value,
            None => return Ok(None),
        };

        let mut fs_file = File::open(self.filename).await?;
        fs_file.seek(SeekFrom::Start((*offset).into())).await?;

        Ok(Some(ZipEntryReader {
            entry: entry.clone(),
            reader: Box::new(fs_file),
        }))
    }
}
