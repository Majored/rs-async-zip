// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use async_zip::{Compression, ZipEntryBuilder, ZipString};
use futures_lite::AsyncWriteExt;

mod common;

#[cfg(feature = "zstd")]
#[tokio::test]
async fn zip_zstd_in_out() {
    let zip_data = common::compress_to_mem(Compression::Zstd).await;
    common::check_decompress_mem(zip_data).await
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn zip_decompress_in_out() {
    let zip_data = common::compress_to_mem(Compression::Deflate).await;
    common::check_decompress_mem(zip_data).await
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn zip32_stream_deflate_sizes() {
    use async_zip::base::write::ZipFileWriter;
    use std::io::{Cursor, Read};

    let content = b"async_zip ZIP32 streaming size regression\n".repeat(100);
    let mut writer = ZipFileWriter::new(Vec::new()).force_no_zip64();
    let opts = ZipEntryBuilder::new("payload.txt".into(), Compression::Deflate);
    let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
    for chunk in content.chunks(512) {
        entry_writer.write_all(chunk).await.unwrap();
    }
    entry_writer.close().await.unwrap();
    let zip_bytes = writer.close().await.unwrap();

    let mut archive = zip::ZipArchive::new(Cursor::new(&zip_bytes)).unwrap();
    // The single ZIP32 entry's 16-byte data descriptor immediately precedes the central directory.
    let descriptor_offset = archive.central_directory_start() as usize - 16;
    let mut file = archive.by_name("payload.txt").unwrap();
    let compressed_size = descriptor_offset as u64 - file.data_start();
    assert!(compressed_size < content.len() as u64);
    assert_eq!(file.compressed_size(), compressed_size);
    assert_eq!(file.size(), content.len() as u64);
    assert_eq!(file.crc32(), crc32fast::hash(&content));

    // ZIP32 data descriptors store CRC-32, compressed size, then uncompressed size.
    let mut descriptor = Vec::new();
    descriptor.extend_from_slice(&0x08074b50u32.to_le_bytes());
    descriptor.extend_from_slice(&crc32fast::hash(&content).to_le_bytes());
    descriptor.extend_from_slice(&(compressed_size as u32).to_le_bytes());
    descriptor.extend_from_slice(&(content.len() as u32).to_le_bytes());
    assert_eq!(&zip_bytes[descriptor_offset..descriptor_offset + descriptor.len()], descriptor.as_slice());

    let mut extracted = Vec::new();
    file.read_to_end(&mut extracted).unwrap();
    assert_eq!(extracted, content);
}

#[tokio::test]
async fn zip_store_in_out() {
    let zip_data = common::compress_to_mem(Compression::Stored).await;
    common::check_decompress_mem(zip_data).await
}

#[tokio::test]
async fn zip_utf8_extra_in_out_stream() {
    let mut zip_bytes = Vec::with_capacity(10_000);

    {
        // writing
        let content = "Test".as_bytes();
        let mut writer = async_zip::base::write::ZipFileWriter::new(&mut zip_bytes);
        let filename =
            ZipString::new_with_alternative("\u{4E2D}\u{6587}.txt".to_string(), b"\xD6\xD0\xCe\xC4.txt".to_vec());
        let opts = ZipEntryBuilder::new(filename, Compression::Stored);

        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        entry_writer.write_all(content).await.unwrap();
        entry_writer.close().await.unwrap();

        writer.close().await.unwrap();
    }

    {
        // reading
        let zip = async_zip::base::read::mem::ZipFileReader::new(zip_bytes).await.unwrap();
        let zip_entries: Vec<_> = zip.file().entries().to_vec();
        assert_eq!(zip_entries.len(), 1);
        assert_eq!(zip_entries[0].filename().as_str().unwrap(), "\u{4E2D}\u{6587}.txt");
        assert_eq!(zip_entries[0].filename().alternative(), Some(b"\xD6\xD0\xCe\xC4.txt".as_ref()));
    }
}

#[tokio::test]
async fn zip_utf8_extra_in_out_whole() {
    let mut zip_bytes = Vec::with_capacity(10_000);

    {
        // writing
        let content = "Test".as_bytes();
        let mut writer = async_zip::base::write::ZipFileWriter::new(&mut zip_bytes);
        let filename =
            ZipString::new_with_alternative("\u{4E2D}\u{6587}.txt".to_string(), b"\xD6\xD0\xCe\xC4.txt".to_vec());
        let opts = ZipEntryBuilder::new(filename, Compression::Stored);
        writer.write_entry_whole(opts, content).await.unwrap();
        writer.close().await.unwrap();
    }

    {
        // reading
        let zip = async_zip::base::read::mem::ZipFileReader::new(zip_bytes).await.unwrap();
        let zip_entries: Vec<_> = zip.file().entries().to_vec();
        assert_eq!(zip_entries.len(), 1);
        assert_eq!(zip_entries[0].filename().as_str().unwrap(), "\u{4E2D}\u{6587}.txt");
        assert_eq!(zip_entries[0].filename().alternative(), Some(b"\xD6\xD0\xCe\xC4.txt".as_ref()));
    }
}
