// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[tokio::test]
async fn single_file_deflate() {
    use crate::base::read1::seek::ZipArchiveReader;
    use futures_lite::io::Cursor;
    use futures_lite::AsyncReadExt;

    let data = Cursor::new(&include_bytes!("macos_single_file_deflate.zip"));

    let mut archive_reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");
    let mut find_iter = archive_reader.find(b"hello.txt").expect("we've loaded cdrs");
    let file_index = find_iter.next().expect("failed to find file");
    assert!(find_iter.next().is_none());

    drop(find_iter);
    let mut file_reader = archive_reader.file(file_index).await.expect("failed to get file reader");

    let mut read_data = String::new();
    file_reader.read_to_string(&mut read_data).await.expect("read failed");
}
