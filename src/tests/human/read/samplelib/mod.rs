// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// https://samplelib.com/license.html

use crate::base::read1::seek::ZipArchiveReader;
use futures_lite::AsyncReadExt;
use futures_lite::io::Cursor;

#[tokio::test]
async fn empty() {
    let data = Cursor::new(&include_bytes!("sample-empty.zip"));
    let archive_reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");

    assert_eq!(archive_reader.ceocdr().is_zip64(), false);
    assert_eq!(archive_reader.ceocdr().cd_size().expect("valid zip64-aware cd size"), 0);
    assert_eq!(archive_reader.cdrs().len(), 0);
}

#[tokio::test]
async fn simple() {
    let data = Cursor::new(&include_bytes!("sample-simple.zip"));
    let file_data = include_bytes!("sample-simple/hello.txt");

    let mut archive_reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
   
    assert_eq!(archive_reader.ceocdr().is_zip64(), false);
    assert_eq!(archive_reader.cdrs().len(), 1);
    // TODO: comment

    let mut iter = archive_reader.find(b"hello.txt").expect("we've loaded cdrs");
    let index = iter.next().expect("failed to find file");
    drop(iter);
    let mut file_reader = archive_reader.file(index).await.expect("failed to get file reader");

    let mut buffer = String::new();
    file_reader.read_to_string(&mut buffer).await.expect("failed to read file");
    assert_eq!(buffer.as_bytes(), file_data);
}

#[tokio::test]
async fn with_html() {
    let data = Cursor::new(&include_bytes!("sample-with-html.zip"));
    let file_data = include_bytes!("sample-with-html/index.html");

    let mut archive_reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
   
    assert_eq!(archive_reader.ceocdr().is_zip64(), false);
    assert_eq!(archive_reader.ceocdr().num_entries().expect("valid zip64 aware num of entries"), 1);
    assert_eq!(archive_reader.cdrs().len(), 1);

    let mut iter = archive_reader.find(b"index.html").expect("we've loaded cdrs");
    let index = iter.next().expect("failed to find file");
    drop(iter);
    let mut file_reader = archive_reader.file(index).await.expect("failed to get file reader");

    let mut buffer = String::new();
    file_reader.read_to_string(&mut buffer).await.expect("failed to read file");
    assert_eq!(buffer.as_bytes(), file_data);
}
