use tokio::io::AsyncReadExt;

use crate::read::mem::ZipFileReader;

/// Tests opening and reading a zip64 archive.
/// It contains one file named "-" with a zip 64 extended field header.
#[tokio::test]
async fn test_read_zip64_archive() {
    let data = include_bytes!("zip64.zip").to_vec();

    let reader = ZipFileReader::new(data).await.unwrap();
    let mut entry_reader = reader.entry(0).await.unwrap();

    let mut read_data = String::new();
    entry_reader.read_to_string(&mut read_data).await.expect("read failed");

    assert_eq!(read_data, "Hello World!");
}
