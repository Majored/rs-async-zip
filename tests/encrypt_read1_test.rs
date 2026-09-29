// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! Round-trip tests for ZipCrypto-encrypted files read through the rewritten read module
//! ([`async_zip::base::read1`]). Archives are written by this crate (whole-entry and streaming
//! writers) and, on Unix, by Info-ZIP's `zip`, then decrypted via both the seeking and the
//! streaming reader.

#![cfg(feature = "deflate")]

use async_zip::base::read1::seek::ZipArchiveReader as SeekArchiveReader;
use async_zip::base::read1::stream::ZipArchiveReader as StreamArchiveReader;
use async_zip::base::read1::ZipOptions;
use async_zip::base::write::ZipFileWriter;
use async_zip::error::ZipError;
use async_zip::{Compression, ZipEntryBuilder};
use futures_lite::io::{AsyncReadExt, AsyncWriteExt, Cursor};

const PASSWORD: &[u8] = b"read1-decryption-test";
const PASSWORD_STR: &str = "read1-decryption-test";

/// Deterministic pseudo-random sample data (xorshift32).
fn sample_data(len: usize) -> Vec<u8> {
    let mut x: u32 = 0x12345678;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            ((x >> 24) & 0xFF) as u8
        })
        .collect()
}

fn opts_with_password(password: Option<&[u8]>) -> ZipOptions {
    ZipOptions { password: password.map(|p| p.to_vec()), ..Default::default() }
}

/// Writes a single password-protected entry, either via the whole-entry or the streaming writer.
async fn write_encrypted(compression: Compression, streamed: bool) -> Vec<u8> {
    let data = sample_data(4096);
    let mut bytes = Vec::new();
    {
        let mut writer = ZipFileWriter::new(&mut bytes);
        let builder = ZipEntryBuilder::new("enc.bin".into(), compression).password(PASSWORD.to_vec());
        if streamed {
            let mut entry = writer.write_entry_stream(builder).await.unwrap();
            entry.write_all(&data).await.unwrap();
            entry.close().await.unwrap();
        } else {
            writer.write_entry_whole(builder, &data).await.unwrap();
        }
        writer.close().await.unwrap();
    }
    bytes
}

/// Writes a single unencrypted deflate entry.
async fn write_plain() -> Vec<u8> {
    let data = sample_data(1024);
    let mut bytes = Vec::new();
    {
        let mut writer = ZipFileWriter::new(&mut bytes);
        writer.write_entry_whole(ZipEntryBuilder::new("plain.bin".into(), Compression::Deflate), &data).await.unwrap();
        writer.close().await.unwrap();
    }
    bytes
}

async fn read_seek(bytes: Vec<u8>, password: Option<&[u8]>) -> Result<Vec<u8>, ZipError> {
    let mut reader = SeekArchiveReader::open_with_options(Cursor::new(bytes), opts_with_password(password)).await?;
    let mut file = reader.file(0).await?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).await?;
    Ok(out)
}

async fn read_stream(bytes: Vec<u8>, password: Option<&[u8]>) -> Result<Vec<u8>, ZipError> {
    let mut archive = StreamArchiveReader::new_with_options(Cursor::new(bytes), opts_with_password(password));
    let mut out = Vec::new();
    while let Some(file) = archive.next().await? {
        file.read_to_end(&mut out).await?;
    }
    Ok(out)
}

#[tokio::test]
async fn seek_reads_whole_encrypted_deflate() {
    let data = sample_data(4096);
    let bytes = write_encrypted(Compression::Deflate, false).await;
    assert_eq!(read_seek(bytes, Some(PASSWORD)).await.unwrap(), data);
}

#[tokio::test]
async fn seek_reads_whole_encrypted_stored() {
    let data = sample_data(4096);
    let bytes = write_encrypted(Compression::Stored, false).await;
    assert_eq!(read_seek(bytes, Some(PASSWORD)).await.unwrap(), data);
}

#[tokio::test]
async fn stream_reads_whole_encrypted_deflate() {
    let data = sample_data(4096);
    let bytes = write_encrypted(Compression::Deflate, false).await;
    assert_eq!(read_stream(bytes, Some(PASSWORD)).await.unwrap(), data);
}

/// Streaming writes set the data-descriptor flag, so the encryption header check byte is derived
/// from the DOS time rather than the CRC. The seeking reader must accept those too.
#[tokio::test]
async fn seek_reads_streamed_encrypted_deflate() {
    let data = sample_data(4096);
    let bytes = write_encrypted(Compression::Deflate, true).await;
    assert_eq!(read_seek(bytes, Some(PASSWORD)).await.unwrap(), data);
}

/// Streaming writes always use data descriptors, which the streaming reader does not support
/// (yet). Encrypted or not, such entries must be read via the seeking reader.
#[tokio::test]
async fn stream_reader_rejects_data_descriptor_entries() {
    let bytes = write_encrypted(Compression::Deflate, true).await;
    let mut archive = StreamArchiveReader::new_with_options(Cursor::new(bytes), opts_with_password(Some(PASSWORD)));
    let err = archive.next().await.err().expect("expected an error");
    assert!(matches!(err, ZipError::FeatureNotSupported("stream reading data descriptors")));
}

#[tokio::test]
async fn encrypted_flag_is_exposed_via_cdr() {
    let encrypted = write_encrypted(Compression::Stored, false).await;
    let reader =
        SeekArchiveReader::open_with_options(Cursor::new(encrypted), opts_with_password(Some(PASSWORD))).await.unwrap();
    assert!(reader.cdrs()[0].cdrh.gpf.encrypted());

    let plain = write_plain().await;
    let reader =
        SeekArchiveReader::open_with_options(Cursor::new(plain), opts_with_password(Some(PASSWORD))).await.unwrap();
    assert!(!reader.cdrs()[0].cdrh.gpf.encrypted());
}

/// Opening an encrypted file without a password must fail before any data is read.
#[tokio::test]
async fn missing_password_reports_password_required() {
    let bytes = write_encrypted(Compression::Deflate, false).await;

    let err = read_seek(bytes.clone(), None).await.unwrap_err();
    assert!(matches!(err, ZipError::PasswordRequired));

    let mut archive = StreamArchiveReader::new_with_options(Cursor::new(bytes), opts_with_password(None));
    let err = archive.next().await.err().expect("expected an error");
    assert!(matches!(err, ZipError::PasswordRequired));
}

/// A wrong password must be detected via the encryption header check byte, on both readers.
#[tokio::test]
async fn wrong_password_reports_invalid_password() {
    let bytes = write_encrypted(Compression::Deflate, false).await;

    let err = read_seek(bytes.clone(), Some(b"wrong-password")).await.unwrap_err();
    assert!(matches!(err, ZipError::UpstreamReadError(e) if e.kind() == std::io::ErrorKind::InvalidData));

    let mut archive =
        StreamArchiveReader::new_with_options(Cursor::new(bytes), opts_with_password(Some(b"wrong-password")));
    let mut out = Vec::new();
    let file = archive.next().await.unwrap().unwrap();
    let err = file.read_to_end(&mut out).await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

/// Setting a password must not affect entries which are not encrypted.
#[tokio::test]
async fn password_options_do_not_affect_plain_entries() {
    let data = sample_data(1024);
    let bytes = write_plain().await;
    assert_eq!(read_seek(bytes, Some(PASSWORD)).await.unwrap(), data);
}

/// Info-ZIP's `zip` is the reference ZipCrypto implementation; archives it encrypts must be
/// readable here, and vice versa (the reverse direction is covered by `encrypt_stream_test.rs`).
#[cfg(unix)]
#[tokio::test]
async fn reads_info_zip_encrypted_archive() {
    let data = sample_data(4096);
    let dir = std::env::temp_dir().join("rs-async-zip-infozip-test");
    std::fs::create_dir_all(&dir).unwrap();

    let src = dir.join("infozip.bin");
    std::fs::write(&src, &data).unwrap();

    let out = dir.join("infozip.zip");
    let _ = std::fs::remove_file(&out);
    let output = std::process::Command::new("zip")
        .current_dir(&dir)
        .args(["-P", PASSWORD_STR, "infozip.zip", "infozip.bin"])
        .output()
        .expect("zip binary not available");
    assert!(output.status.success(), "zip failed:\n{}", String::from_utf8_lossy(&output.stderr));

    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(read_seek(bytes, Some(PASSWORD)).await.unwrap(), data);

    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(read_stream(bytes, Some(PASSWORD)).await.unwrap(), data);
}
