// Copyright (c) 2022-2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

pub(crate) mod combined;
pub(crate) mod read;
pub(crate) mod spec;
pub(crate) mod write;

pub(crate) mod human;

use std::sync::Once;
use futures_lite::io::Cursor;
use futures_lite::AsyncReadExt;

static ENV_LOGGER: Once = Once::new();

/// Initialize the env logger for any tests that require it.
/// Safe to call multiple times.
fn init_logger() {
    ENV_LOGGER.call_once(|| env_logger::Builder::from_default_env().format_module_path(true).init());
}

pub(crate) struct TestArchive {
    pub(crate) data: &'static [u8],
    pub(crate) files: Option<Vec<(&'static [u8], &'static [u8])>>,
    pub(crate) is_zip64: bool,
    pub(crate) cd_size: Option<u64>,
    pub(crate) num_files: usize,
}

pub(crate) async fn exec_test_seek(test_archive: &TestArchive) {
    use crate::base::read1::seek::ZipArchiveReader;

    let data = Cursor::new(test_archive.data);
    let mut archive_reader = ZipArchiveReader::open(data).await.expect("failed to open zip archive");

    assert_eq!(archive_reader.ceocdr().is_zip64(), test_archive.is_zip64);
    assert_eq!(archive_reader.cdrs().len(), test_archive.num_files);

    if let Some(cd_size) = test_archive.cd_size {
        assert_eq!(archive_reader.ceocdr().cd_size().expect("valid zip64-aware cd size"), cd_size);
    }

    if let Some(files) = &test_archive.files {
        for (file_name, file_data) in files {
            let mut iter = archive_reader.find(file_name).expect("we've loaded cdrs");
            let index = iter.next().expect("failed to find file");
            drop(iter);
            let mut file_reader = archive_reader.file(index).await.expect("failed to get file reader");

            let mut buffer = String::new();
            file_reader.read_to_string(&mut buffer).await.expect("failed to read file");
            assert_eq!(buffer.as_bytes(), *file_data);
        }
    }

    for i in 0..archive_reader.cdrs().len() {
        let mut file_reader = archive_reader.file(i).await.expect("failed to get file reader");
        let mut buffer = Vec::new();
        file_reader.read_to_end(&mut buffer).await.expect("failed to read file");
    }
}

pub(crate) async fn exec_test_stream(test_archive: &TestArchive) {
    use crate::base::read1::stream::ZipArchiveReader;

    let data = Cursor::new(test_archive.data);
    let mut archive_reader = ZipArchiveReader::new(data);

    if let Some(files) = &test_archive.files {
        for (file_name, file_data) in files {
            let file_reader = archive_reader.next().await.expect("read next file").expect("next file exists");
            assert_eq!(file_reader.lf().insecure_file_name.as_bytes(), *file_name);

            let mut buffer = String::new();
            file_reader.read_to_string(&mut buffer).await.expect("failed to read file");
            assert_eq!(buffer.as_bytes(), *file_data);
        }

        assert!(archive_reader.next().await.expect("read next file").is_none());
    }

    while let Some(_) = archive_reader.next().await.expect("read next file") {}

    assert_eq!(archive_reader.ceocdr().expect("has ceocdr").is_zip64(), test_archive.is_zip64);
    assert_eq!(archive_reader.cdrs().expect("has cdrs").len(), test_archive.num_files);
}
