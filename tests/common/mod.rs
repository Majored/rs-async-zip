// Copyright (c) 2023 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use async_zip::read::mem;
use async_zip::read::seek;
use async_zip::write::ZipFileWriter;
use async_zip::Compression;
use async_zip::ZipEntryBuilder;
use futures_util::io::AsyncWriteExt;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

const FOLDER_PREFIX: &str = "tests/test_inputs";

const FILE_LIST: &[&str] = &[
    "sample_data/alpha/back_to_front.txt",
    "sample_data/alpha/front_to_back.txt",
    "sample_data/numeric/forward.txt",
    "sample_data/numeric/reverse.txt",
];

pub async fn compress_to_mem(compress: Compression) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10_000);
    let mut writer = ZipFileWriter::new(&mut bytes);

    for fname in FILE_LIST {
        let content = tokio::fs::read(format!("{FOLDER_PREFIX}/{fname}")).await.unwrap();
        let opts = ZipEntryBuilder::new(fname.to_string(), compress);

        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        entry_writer.write_all(&content).await.unwrap();
        entry_writer.close().await.unwrap();
    }
    writer.close().await.unwrap();
    bytes
}

#[cfg(feature = "fs")]
pub async fn check_decompress_fs(fname: &str) {
    use async_zip::read::fs;
    let mut zip = fs::ZipFileReader::new(fname).await.unwrap();
    let zip_entries: Vec<_> = zip.file().entries().into_iter().cloned().collect();
    for (idx, entry) in zip_entries.into_iter().enumerate() {
        if entry.entry().dir() {
            continue;
        }
        let fname = entry.entry().filename();
        let mut output = String::new();
        let mut reader = zip.entry(idx).await.unwrap();
        let _ = reader.read_to_string_checked(&mut output, entry.entry()).await.unwrap();
        let fs_file = format!("{FOLDER_PREFIX}/{fname}");
        let expected = tokio::fs::read_to_string(fs_file).await.unwrap();
        assert_eq!(output, expected, "for {fname}, expect zip data to match file data");
    }
}

pub async fn check_decompress_seek(fname: &str) {
    let file = File::open(fname).await.unwrap();
    let mut file_compat = file.compat();
    let mut zip = seek::ZipFileReader::new(&mut file_compat).await.unwrap();
    let zip_entries: Vec<_> = zip.file().entries().into_iter().cloned().collect();
    for (idx, entry) in zip_entries.into_iter().enumerate() {
        if entry.entry().dir() {
            continue;
        }
        let fname = entry.entry().filename();
        let mut output = String::new();
        let mut reader = zip.entry(idx).await.unwrap();
        let _ = reader.read_to_string_checked(&mut output, entry.entry()).await.unwrap();
        let fs_file = format!("tests/test_inputs/{fname}");
        let expected = tokio::fs::read_to_string(fs_file).await.unwrap();
        assert_eq!(output, expected, "for {fname}, expect zip data to match file data");
    }
}

pub async fn check_decompress_mem(zip_data: Vec<u8>) {
    let zip = mem::ZipFileReader::new(zip_data).await.unwrap();
    let zip_entries: Vec<_> = zip.file().entries().into_iter().cloned().collect();
    for (idx, entry) in zip_entries.into_iter().enumerate() {
        if entry.entry().dir() {
            continue;
        }
        let fname = entry.entry().filename();
        let mut output = String::new();
        let mut reader = zip.entry(idx).await.unwrap();
        let _ = reader.read_to_string_checked(&mut output, entry.entry()).await.unwrap();
        let fs_file = format!("{FOLDER_PREFIX}/{fname}");
        let expected = tokio::fs::read_to_string(fs_file).await.unwrap();
        assert_eq!(output, expected, "for {fname}, expect zip data to match file data");
    }
}
