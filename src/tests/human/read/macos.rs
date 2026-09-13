// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::tests::{TestArchive, exec_test_seek};

#[tokio::test]
#[cfg(feature = "deflate")]
async fn single_file_deflate() {
    let test_archive = TestArchive {
        data: include_bytes!("macos_single_file_deflate.zip"),
        files: Some(vec![(b"hello.txt", b"Hello, this is a stored ZIP entry!")]),
        is_zip64: false,
        cd_size: None,
        num_files: 1,
    };

    exec_test_seek(&test_archive).await;
    // exec_test_stream(&test_archive).await;
}
