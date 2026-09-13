// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::tests::{TestArchive, exec_test_seek, exec_test_stream};

#[tokio::test]
async fn single() {
    let test_archive = TestArchive {
        data: include_bytes!("zip64.zip"),
        files: None,
        is_zip64: true,
        cd_size: None,
        num_files: 1,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
}

#[tokio::test]
async fn many() {
    let test_archive = TestArchive {
        data: include_bytes!("zip64many.zip"),
        files: None,
        is_zip64: true,
        cd_size: None,
        num_files: 65_537,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
}
