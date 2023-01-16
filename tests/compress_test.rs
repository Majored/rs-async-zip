use async_zip::Compression;

mod common;

#[cfg(feature = "zstd")]
#[tokio::test]
async fn zip_zstd_in_out() {
    let zip_data = common::compress_to_mem(Compression::Zstd).await;
    common::check_decompress_mem(zip_data).await
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn zip_decompress_in_out() {
    let zip_data = common::compress_to_mem(Compression::Deflate).await;
    common::check_decompress_mem(zip_data).await
}

#[tokio::test]
async fn zip_store_in_out() {
    let zip_data = common::compress_to_mem(Compression::Stored).await;
    common::check_decompress_mem(zip_data).await
}
