// Copyright Cognite AS, 2023

use crate::error::{Zip64ErrorCase, ZipError};
use crate::spec::consts::NON_ZIP64_MAX_SIZE;
use crate::tests::write::AsyncSink;
use crate::write::ZipFileWriter;
use crate::{Compression, ZipEntryBuilder};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn test_write_zip64_file() {}

/// Tests that when force_no_zip64 is true, EntryWholeWriter errors when trying to write more than
/// u16::MAX files to a single archive.
#[tokio::test]
async fn test_force_no_zip64_errors_with_too_many_files_whole() {
    let mut sink = AsyncSink;
    let mut writer = ZipFileWriter::new(&mut sink).force_no_zip64();
    for i in 0..u16::MAX {
        let entry = ZipEntryBuilder::new(format!("{i}"), Compression::Stored);
        writer.write_entry_whole(entry, &[]).await.unwrap()
    }
    let entry = ZipEntryBuilder::new(format!("65537"), Compression::Stored);
    let result = writer.write_entry_whole(entry, &[]).await;

    assert!(matches!(result, Err(ZipError::Zip64Needed(Zip64ErrorCase::TooManyFiles))));
}

/// Tests that when force_no_zip64 is true, EntryStreamWriter errors when trying to write more than
/// u16::MAX files to a single archive.
#[tokio::test]
async fn test_force_no_zip64_errors_with_too_many_files_stream() {
    let mut sink = AsyncSink;
    let mut writer = ZipFileWriter::new(&mut sink).force_no_zip64();
    for i in 0..u16::MAX {
        let entry = ZipEntryBuilder::new(format!("{i}"), Compression::Stored);
        let entrywriter = writer.write_entry_stream(entry).await.unwrap();
        entrywriter.close().await.unwrap();
    }
    let entry = ZipEntryBuilder::new(format!("65537"), Compression::Stored);
    let entrywriter = writer.write_entry_stream(entry).await.unwrap();
    let result = entrywriter.close().await;

    assert!(matches!(result, Err(ZipError::Zip64Needed(Zip64ErrorCase::TooManyFiles))));
}

const NUM_BATCHES: usize = (NON_ZIP64_MAX_SIZE as u64 / 100_000 + 1) as usize;
#[tokio::test]
async fn test_force_no_zip64_errors_with_too_large_file_stream() {
    let mut sink = AsyncSink;
    let mut writer = ZipFileWriter::new(&mut sink).force_no_zip64();

    let entry = ZipEntryBuilder::new(format!("-"), Compression::Stored);
    let mut entrywriter = writer.write_entry_stream(entry).await.unwrap();

    // Writing 4GB, 1kb at a time
    for _ in 0..NUM_BATCHES {
        entrywriter.write_all(&[0; 100_000]).await.unwrap();
    }
    let result = entrywriter.close().await;

    assert!(matches!(result, Err(ZipError::Zip64Needed(Zip64ErrorCase::LargeFile))));
}
