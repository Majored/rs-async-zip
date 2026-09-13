// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

// https://samplelib.com/license.html

use crate::tests::{TestArchive, exec_test_seek, exec_test_stream};

#[tokio::test]
async fn empty() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-empty.zip"),
        files: None,
        num_files: 0,
        is_zip64: false,
        cd_size: Some(0),
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn simple() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-simple.zip"),
        files: Some(vec![(b"hello.txt", include_bytes!("sample-simple/hello.txt"))]),
        num_files: 1,
        is_zip64: false,
        cd_size: None,
    };

    // TODO: comment
    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn with_html() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-with-html.zip"),
        files: Some(vec![(b"index.html", include_bytes!("sample-with-html/index.html"))]),
        num_files: 1,
        is_zip64: false,
        cd_size: None,
    };
    
    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn nested() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-nested.zip"),
        files: None,
        num_files: 8,
        is_zip64: false,
        cd_size: None,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;

    // TODO: validate contents
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn many_files() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-many-files.zip"),
        files: None,
        num_files: 100,
        is_zip64: false,
        cd_size: None,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
    // TODO: validate contents
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn project() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-project.zip"),
        files: None,
        num_files: 7,
        is_zip64: false,
        cd_size: None,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
    // TODO: validate contents
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn mixed() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-mixed.zip"),
        files: None,
        num_files: 7,
        is_zip64: false,
        cd_size: None,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
    // TODO: validate contents
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn size_1mb() {
    let test_archive = TestArchive {
        data: include_bytes!("sample-1mb.zip"),
        files: None,
        num_files: 5,
        is_zip64: false,
        cd_size: None,
    };

    exec_test_seek(&test_archive).await;
    exec_test_stream(&test_archive).await;
    // TODO: validate contents
}
