mod common;

const ZSTD_ZIP_FILE: &str = "tests/test_inputs/sample_data.zstd.zip";
const DEFLATE_ZIP_FILE: &str = "tests/test_inputs/sample_data.deflate.zip";
const STORE_ZIP_FILE: &str = "tests/test_inputs/sample_data.store.zip";

#[cfg(feature = "zstd")]
#[tokio::test]
async fn decompress_zstd_zip_seek() {
    common::check_decompress_seek(ZSTD_ZIP_FILE).await
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn decompress_deflate_zip_seek() {
    common::check_decompress_seek(DEFLATE_ZIP_FILE).await
}

#[tokio::test]
async fn decompress_store_zip_seek() {
    common::check_decompress_seek(STORE_ZIP_FILE).await
}

#[cfg(feature = "zstd")]
#[tokio::test]
async fn decompress_zstd_zip_mem() {
    let content = tokio::fs::read(ZSTD_ZIP_FILE).await.unwrap();
    common::check_decompress_mem(content).await
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn decompress_deflate_zip_mem() {
    let content = tokio::fs::read(DEFLATE_ZIP_FILE).await.unwrap();
    common::check_decompress_mem(content).await
}

#[tokio::test]
async fn decompress_store_zip_mem() {
    let content = tokio::fs::read(STORE_ZIP_FILE).await.unwrap();
    common::check_decompress_mem(content).await
}

#[cfg(feature = "zstd")]
#[cfg(feature = "fs")]
#[tokio::test]
async fn decompress_zstd_zip_fs() {
    common::check_decompress_fs(ZSTD_ZIP_FILE).await
}

#[cfg(feature = "deflate")]
#[cfg(feature = "fs")]
#[tokio::test]
async fn decompress_deflate_zip_fs() {
    common::check_decompress_fs(DEFLATE_ZIP_FILE).await
}

#[cfg(feature = "fs")]
#[tokio::test]
async fn decompress_store_zip_fs() {
    common::check_decompress_fs(STORE_ZIP_FILE).await
}
