use tokio::io::AsyncReadExt;

use crate::read::mem::ZipFileReader;

const ZIP64_ZIP_CONTENTS: &str = "Hello World!\n";

/// Tests opening and reading a zip64 archive.
/// It contains one file named "-" with a zip 64 extended field header.
#[tokio::test]
async fn test_read_zip64_archive() {
    env_logger::init();

    let data = include_bytes!("zip64.zip").to_vec();

    let reader = ZipFileReader::new(data).await.unwrap();
    let mut entry_reader = reader.entry(0).await.unwrap();

    let mut read_data = String::new();
    entry_reader.read_to_string(&mut read_data).await.expect("read failed");

    assert_eq!(
        read_data.chars().count(),
        ZIP64_ZIP_CONTENTS.chars().count(),
        "{read_data:?} != {ZIP64_ZIP_CONTENTS:?}"
    );
    assert_eq!(read_data, ZIP64_ZIP_CONTENTS);
}
