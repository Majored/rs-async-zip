use tokio::io::AsyncReadExt;

const ZIP64_ZIP_CONTENTS: &str = "Hello World!\n";

/// Tests opening and reading a zip64 archive.
/// It contains one file named "-" with a zip 64 extended field header.
#[tokio::test]
async fn test_read_zip64_archive_mem() {
    use crate::read::mem::ZipFileReader;

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

/// Like test_read_zip64_archive_mem() but for the streaming version
#[tokio::test]
async fn test_read_zip64_archive_stream() {
    use crate::read::stream::ZipFileReader;

    env_logger::init();

    let data = include_bytes!("zip64.zip").to_vec();

    let reader = ZipFileReader::new(data.as_slice());
    let mut entry_reader = reader.next_entry().await.unwrap().unwrap();

    let mut read_data = String::new();
    entry_reader.reader().read_to_string(&mut read_data).await.expect("read failed");

    assert_eq!(
        read_data.chars().count(),
        ZIP64_ZIP_CONTENTS.chars().count(),
        "{read_data:?} != {ZIP64_ZIP_CONTENTS:?}"
    );
    assert_eq!(read_data, ZIP64_ZIP_CONTENTS);
}
