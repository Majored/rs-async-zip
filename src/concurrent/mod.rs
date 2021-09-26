// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! A module which supports reading ZIP file entries concurrently.
//!
//! # Note
//! This implementation requires the caller provide a filename rather than a mutable reference to a reader. This is so
//! that multiple entries may read from concurrently open files in order to seek+read independently. Further, this
//! implementation only supports concurrent reading, as writing would require a high degree of synchronisation which
//! would provide little, if any benefit.
//!
//! # Example

use crate::header::CentralDirectoryHeader;
use crate::error::Result;

use std::collections::HashMap;

use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use std::io::SeekFrom;

pub struct ZipStreamReader<'a> {
    pub(crate) file_name: &'a str,
    pub(crate) entries: HashMap<String, CentralDirectoryHeader>
}

impl<'a> ZipStreamReader<'a> {
    pub async fn new(file_name: &'a str) -> Result<ZipStreamReader<'a>> {
        let file = ZipStreamReader {
            file_name,
            entries: HashMap::default(),
        };

        let mut fs_file = File::open(file_name).await?;
        let len = fs_file.metadata().await?.len();
        let mut seek_to = 0;

        if len > 65557 {
            seek_to = len - 65557;
        }

        fs_file.seek(SeekFrom::Start(seek_to)).await?;

        // TODO:
        // Find end of central directory, seek to start of central directory, read all entries.

        Ok(file)
    }

    pub async fn get<'b>(&'b self, entry: &str) -> Result<Option<ZipEntry<'b>>> {
        let header = match self.entries.get(entry) {
            Some(header) => header,
            None => return Ok(None),
        }; 
        
        let mut fs_file = File::open(&self.file_name).await?;
        fs_file.seek(SeekFrom::Start(header.lh_offset.into())).await?;

        // TODO:
        // Read local file header and position at start of data.

        Ok(Some(ZipEntry {
            header,
            file: fs_file,
        }))
    }
}

pub struct ZipEntry<'b> {
    header: &'b CentralDirectoryHeader,
    file: File,
}