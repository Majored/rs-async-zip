// Copyright Cognite AS, 2023

use crate::error::{Zip64ErrorCase, ZipError};
use crate::spec::consts::NON_ZIP64_MAX_SIZE;
use crate::tests::init_logger;
use crate::tests::write::AsyncSink;
use crate::write::ZipFileWriter;
use crate::{Compression, ZipEntryBuilder};
use std::io::{Read, Write};

use tokio::io::AsyncWriteExt;

/// Test writing a small zip64 file. No zip64 extra fields would be emitted, but z64 end of directory
/// records should be.
#[tokio::test]
async fn test_write_zip64_file() {
    init_logger();

    let mut buffer = Vec::new();
    let mut writer = ZipFileWriter::new(&mut buffer).force_zip64();
    let entry = ZipEntryBuilder::new("file1".to_string(), Compression::Stored);
    writer.write_entry_whole(entry, &[0, 0, 0, 0]).await.unwrap();
    let entry = ZipEntryBuilder::new("file2".to_string(), Compression::Stored);
    let mut entry_writer = writer.write_entry_stream(entry).await.unwrap();
    entry_writer.write_all(&[0, 0, 0, 0]).await.unwrap();
    entry_writer.close().await.unwrap();
    writer.close().await.unwrap();

    let cursor = std::io::Cursor::new(buffer);
    let mut zip = zip::read::ZipArchive::new(cursor).unwrap();
    let mut file1 = zip.by_name("file1").unwrap();
    assert_eq!(file1.extra_data(), vec![]);
    let mut buffer = Vec::new();
    file1.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer.as_slice(), &[0, 0, 0, 0]);
    drop(file1);

    let mut file2 = zip.by_name("file2").unwrap();
    assert_eq!(file2.extra_data(), vec![]);
    let mut buffer = Vec::new();
    file2.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer.as_slice(), &[0, 0, 0, 0]);
}

/// Test writing a zip64 file with more than u16::MAX files.
#[tokio::test]
async fn test_write_zip64_file_many_entries() {
    init_logger();

    // The generated file will likely be ~3MB in size.
    let mut buffer = Vec::with_capacity(3_500_000);

    let mut writer = ZipFileWriter::new(&mut buffer);
    for i in 0..=u16::MAX as u32 + 1 {
        let entry = ZipEntryBuilder::new(i.to_string(), Compression::Stored);
        writer.write_entry_whole(entry, &[]).await.unwrap();
    }
    assert!(writer.is_zip64);
    writer.close().await.unwrap();

    let cursor = std::io::Cursor::new(buffer);
    let mut zip = zip::read::ZipArchive::new(cursor).unwrap();
    assert_eq!(zip.len(), u16::MAX as usize + 2);

    for i in 0..=u16::MAX as u32 + 1 {
        zip.by_name(&i.to_string()).unwrap();
    }
}

/// Tests that EntryWholeWriter switches to Zip64 mode when writing too many files for a non-Zip64.
#[tokio::test]
async fn test_zip64_when_many_files_whole() {
    let mut sink = AsyncSink;
    let mut writer = ZipFileWriter::new(&mut sink);
    for i in 0..=u16::MAX as u32 + 1 {
        let entry = ZipEntryBuilder::new(format!("{i}"), Compression::Stored);
        writer.write_entry_whole(entry, &[]).await.unwrap()
    }
    assert!(writer.is_zip64);
    writer.close().await.unwrap();
}

/// Tests that EntryStreamWriter switches to Zip64 mode when writing too many files for a non-Zip64.
#[tokio::test]
async fn test_zip64_when_many_files_stream() {
    let mut sink = AsyncSink;
    let mut writer = ZipFileWriter::new(&mut sink);
    for i in 0..=u16::MAX as u32 + 1 {
        let entry = ZipEntryBuilder::new(format!("{i}"), Compression::Stored);
        let entrywriter = writer.write_entry_stream(entry).await.unwrap();
        entrywriter.close().await.unwrap();
    }

    assert!(writer.is_zip64);
    writer.close().await.unwrap();
}

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
/// Tests that when force_no_zip64 is true, EntryStreamWriter errors when trying to write
/// a file larger than ~4 GiB to an archive.
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
