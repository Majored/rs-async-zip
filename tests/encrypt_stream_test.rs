// Copyright (c) 2024 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! Round-trip tests for streaming entries written with ZipCrypto encryption.
//!
//! Each test writes entries via `write_entry_stream` / `write_entry_stream_precompressed`
//! (in multiple uneven chunks, like a large file would be streamed) and verifies the result
//! via several independent readers:
//! - the `zip` crate (dev-dependency) with password-based decryption and CRC verification,
//! - this crate's own read path plus the public `ZipCrypto` primitive for manual decryption,
//! - the system `unzip` and Python's `zipfile` module (see dedicated tests below).

#![cfg(feature = "deflate")]
#![allow(deprecated)]

use async_zip::base::read::mem::ZipFileReader;
use async_zip::base::write::compress;
use async_zip::base::write::crc32;
use async_zip::base::write::ZipFileWriter;
use async_zip::crypto::ZipCrypto;
use async_zip::{Compression, ZipEntryBuilder};
use futures_lite::io::{AsyncReadExt, AsyncWriteExt};

use std::io::Read;
use std::io::Write;

const PASSWORD: &[u8] = b"stream-encryption-test";
const PASSWORD_STR: &str = "stream-encryption-test";

const DEFLATE_ENTRY: &str = "deflate_stream.bin";
const STORED_ENTRY: &str = "stored_stream.bin";
const PRECOMP_ENTRY: &str = "deflate_precompressed.bin";

/// Uneven chunking plan summing to 1024, simulating streamed writes of a large file.
const CHUNK_PLAN: &[usize] = &[1, 42, 300, 7, 128, 256, 290];

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

/// Writes `data` through the entry writer in uneven chunks according to `plan`.
async fn write_chunked<W: futures_lite::io::AsyncWrite + Unpin>(
    writer: &mut async_zip::base::write::EntryStreamWriter<'_, W>,
    data: &[u8],
    plan: &[usize],
) {
    let mut pos = 0;
    for &chunk in plan {
        writer.write_all(&data[pos..pos + chunk]).await.unwrap();
        pos += chunk;
    }
    if pos < data.len() {
        writer.write_all(&data[pos..]).await.unwrap();
    }
}

/// Locates the offset of an entry's data region by parsing its local file header.
fn lfh_data_offset(bytes: &[u8], header_offset: u64) -> usize {
    let p = header_offset as usize;
    assert_eq!(&bytes[p..p + 4], &0x04034b50u32.to_le_bytes(), "LFH signature expected");
    // LFH layout: signature (4) + fixed fields (26) with name/extra lengths at +26/+28,
    // then filename, then extra, then the data region.
    let name_len = u16::from_le_bytes([bytes[p + 26], bytes[p + 27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[p + 28], bytes[p + 29]]) as usize;
    p + 30 + name_len + extra_len
}

/// Decrypts a `12-byte header || ciphertext` region with a fresh cipher, returning the plaintext.
fn decrypt_zip_crypto(region: &[u8], password: &[u8]) -> Vec<u8> {
    let mut crypto = ZipCrypto::new(password);
    region.iter().map(|&b| crypto.decrypt_byte(b)).collect()
}

/// Writes one deflate-streaming, one stored-streaming and one deflate-precompressed entry,
/// all password-protected, into a fresh in-memory archive. Returns the archive bytes and the
/// plaintext of each entry.
async fn build_mixed_encrypted_zip() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let deflate_data = sample_data(1024);
    let stored_data = sample_data(768);
    let precomp_data = sample_data(2048);

    let mut bytes = Vec::new();
    let mut writer = ZipFileWriter::new(&mut bytes);

    {
        let opts = ZipEntryBuilder::new(DEFLATE_ENTRY.into(), Compression::Deflate).password(PASSWORD.to_vec());
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &deflate_data, CHUNK_PLAN).await;
        entry_writer.close().await.unwrap();
    }

    {
        let opts = ZipEntryBuilder::new(STORED_ENTRY.into(), Compression::Stored).password(PASSWORD.to_vec());
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &stored_data, &[100, 1, 667]).await;
        entry_writer.close().await.unwrap();
    }

    {
        let opts = ZipEntryBuilder::new(PRECOMP_ENTRY.into(), Compression::Deflate).password(PASSWORD.to_vec());
        let crc = crc32(&precomp_data);
        let compressed = compress(opts.current(), &precomp_data).await;
        let opts =
            opts.crc32(crc).compressed_size(compressed.len() as u64).uncompressed_size(precomp_data.len() as u64);
        let mut entry_writer = writer.write_entry_stream_precompressed(opts).await.unwrap();
        write_chunked(&mut entry_writer, &compressed, &[1, 500, 13]).await;
        entry_writer.close().await.unwrap();
    }

    writer.close().await.unwrap();
    (bytes, deflate_data, stored_data, precomp_data)
}

/// Reads entry `index` of an encrypted archive via the `zip` crate with the password,
/// returning the decrypted and decompressed plaintext (CRC is verified by the crate).
fn read_via_zip_crate(bytes: &[u8], index: usize) -> Vec<u8> {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut file = archive.by_index_decrypt(index, PASSWORD).unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    out
}

/// Deflate streaming + password round-trip, verified by the `zip` crate and by manual
/// decryption of the ciphertext located through this crate's own read path.
#[tokio::test]
async fn zip_deflate_stream_encrypted_roundtrip() {
    let data = sample_data(1024);

    let mut enc_bytes = Vec::new();
    {
        let mut writer = ZipFileWriter::new(&mut enc_bytes);
        let opts = ZipEntryBuilder::new("deflate.bin".into(), Compression::Deflate).password(PASSWORD.to_vec());
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &data, CHUNK_PLAN).await;
        entry_writer.close().await.unwrap();
        writer.close().await.unwrap();
    }

    // Independent reader: zip crate decrypts, inflates and checks the CRC.
    assert_eq!(read_via_zip_crate(&enc_bytes, 0), data);

    // This crate's read path: locate the encrypted region and decrypt it manually; the
    // plaintext must be the same deflate stream an unencrypted (control) entry produces.
    let mut plain_bytes = Vec::new();
    {
        let mut writer = ZipFileWriter::new(&mut plain_bytes);
        let opts = ZipEntryBuilder::new("deflate.bin".into(), Compression::Deflate);
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &data, CHUNK_PLAN).await;
        entry_writer.close().await.unwrap();
        writer.close().await.unwrap();
    }

    let enc_reader = ZipFileReader::new(enc_bytes.clone()).await.unwrap();
    let enc_entry = &enc_reader.file().entries()[0];
    assert!(enc_entry.is_encrypted());
    let enc_offset = lfh_data_offset(enc_reader.data(), enc_entry.header_offset());
    let enc_region = &enc_reader.data()[enc_offset..enc_offset + enc_entry.compressed_size() as usize];
    let decrypted_deflate = decrypt_zip_crypto(enc_region, PASSWORD);

    let plain_reader = ZipFileReader::new(plain_bytes.clone()).await.unwrap();
    let plain_entry = &plain_reader.file().entries()[0];
    assert!(!plain_entry.is_encrypted());
    let plain_offset = lfh_data_offset(plain_reader.data(), plain_entry.header_offset());
    let plain_deflate = &plain_reader.data()[plain_offset..plain_offset + plain_entry.compressed_size() as usize];

    assert_eq!(&decrypted_deflate[12..], plain_deflate, "decrypted body must equal the unencrypted deflate stream");
    assert_eq!(enc_entry.compressed_size(), plain_entry.compressed_size() + 12);
}

/// Stored streaming + password round-trip: the entry reader (Stored passes data through)
/// yields the raw encrypted region, which decrypts to the original plaintext.
#[tokio::test]
async fn zip_stored_stream_encrypted_roundtrip() {
    let data = sample_data(768);

    let mut bytes = Vec::new();
    {
        let mut writer = ZipFileWriter::new(&mut bytes);
        let opts = ZipEntryBuilder::new("stored.bin".into(), Compression::Stored).password(PASSWORD.to_vec());
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &data, &[100, 1, 667]).await;
        entry_writer.close().await.unwrap();
        writer.close().await.unwrap();
    }

    // Independent reader: zip crate decrypts and checks the CRC.
    assert_eq!(read_via_zip_crate(&bytes, 0), data);

    // This crate's read path: Stored entries pass the raw bytes through, so the reader
    // yields the 12-byte encryption header followed by the ciphertext.
    let reader = ZipFileReader::new(bytes.clone()).await.unwrap();
    let mut entry_reader = reader.reader_without_entry(0).await.unwrap();
    let mut raw = Vec::new();
    entry_reader.read_to_end(&mut raw).await.unwrap();
    assert_eq!(raw.len(), data.len() + 12);
    assert_eq!(&decrypt_zip_crypto(&raw, PASSWORD)[12..], &data[..]);
}

/// Precompressed streaming + password round-trip: the caller supplies the already-deflated
/// data plus its CRC, the entry is written with `write_entry_stream_precompressed`.
#[tokio::test]
async fn zip_deflate_precompressed_stream_encrypted_roundtrip() {
    let data = sample_data(2048);

    let mut bytes = Vec::new();
    let crc = crc32(&data);
    {
        let mut writer = ZipFileWriter::new(&mut bytes);
        let opts = ZipEntryBuilder::new("precomp.bin".into(), Compression::Deflate).password(PASSWORD.to_vec());
        let compressed = compress(opts.current(), &data).await;
        let opts = opts.crc32(crc).compressed_size(compressed.len() as u64).uncompressed_size(data.len() as u64);
        let mut entry_writer = writer.write_entry_stream_precompressed(opts).await.unwrap();
        write_chunked(&mut entry_writer, &compressed, &[1, 500, 13]).await;
        entry_writer.close().await.unwrap();
        writer.close().await.unwrap();
    }

    // Independent reader: zip crate decrypts, inflates and checks the CRC.
    assert_eq!(read_via_zip_crate(&bytes, 0), data);

    // This crate's read path: the encrypted region decrypts to exactly the precompressed
    // deflate stream the caller passed in.
    let reader = ZipFileReader::new(bytes.clone()).await.unwrap();
    let entry = &reader.file().entries()[0];
    assert!(entry.is_encrypted());
    let offset = lfh_data_offset(reader.data(), entry.header_offset());
    let region = &reader.data()[offset..offset + entry.compressed_size() as usize];
    let decrypted = decrypt_zip_crypto(region, PASSWORD);
    let expected = compress(ZipEntryBuilder::new("precomp.bin".into(), Compression::Deflate).current(), &data).await;
    assert_eq!(&decrypted[12..], &expected);
}

/// compressed_size accounting: the encrypted entry's size covers the 12-byte encryption
/// header plus the ciphertext body, while the unencrypted entry has no extra bytes.
#[tokio::test]
async fn stream_encrypted_compressed_size_accounting() {
    async fn build(password: bool, compression: Compression) -> (u64, bool) {
        let data = sample_data(1024);
        let mut bytes = Vec::new();
        let mut writer = ZipFileWriter::new(&mut bytes);
        let opts = ZipEntryBuilder::new("entry.bin".into(), compression);
        let opts = if password { opts.password(PASSWORD.to_vec()) } else { opts };
        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        write_chunked(&mut entry_writer, &data, CHUNK_PLAN).await;
        entry_writer.close().await.unwrap();
        writer.close().await.unwrap();
        let reader = ZipFileReader::new(bytes).await.unwrap();
        (reader.file().entries()[0].compressed_size(), reader.file().entries()[0].is_encrypted())
    }

    // Stored is fully deterministic: header (12) + data (sample_data(1024) inside `build`).
    let (enc_size, enc_flag) = build(true, Compression::Stored).await;
    let (plain_size, plain_flag) = build(false, Compression::Stored).await;
    assert!(enc_flag);
    assert!(!plain_flag);
    assert_eq!(enc_size, 1024 + 12);
    assert_eq!(plain_size, 1024);

    // Deflate: encrypted size is exactly 12 bytes above the unencrypted control.
    let (enc_size, _) = build(true, Compression::Deflate).await;
    let (plain_size, _) = build(false, Compression::Deflate).await;
    assert_eq!(enc_size, plain_size + 12);
}

/// System `unzip -P <pwd> -t` must accept the whole encrypted archive (written to /tmp so
/// it can also be inspected manually).
#[cfg(unix)]
#[tokio::test]
async fn unzip_system_verify_stream_encrypted() {
    let (bytes, ..) = build_mixed_encrypted_zip().await;
    let path = std::path::PathBuf::from("/tmp/rs-async-zip-enc-stream.zip");
    std::fs::write(&path, &bytes).unwrap();

    let output = std::process::Command::new("unzip")
        .arg("-P")
        .arg(PASSWORD_STR)
        .arg("-t")
        .arg(&path)
        .output()
        .expect("unzip binary not available");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("No errors detected"),
        "unzip -t failed:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Python's `zipfile` module must read every streaming-encrypted entry with the password and
/// yield the original content (length and CRC32 compared on the Rust side).
#[cfg(unix)]
#[tokio::test]
async fn python_zipfile_verify_stream_encrypted() {
    let (bytes, deflate_data, stored_data, precomp_data) = build_mixed_encrypted_zip().await;
    let path = std::path::PathBuf::from("/tmp/rs-async-zip-enc-stream-py.zip");
    std::fs::write(&path, &bytes).unwrap();

    let script = r#"
import sys, zipfile, zlib
zf = zipfile.ZipFile(sys.argv[1])
pwd = sys.argv[2].encode()
for name in sys.argv[3:]:
    with zf.open(name, pwd=pwd) as f:
        data = f.read()
    print(name, len(data), zlib.crc32(data) & 0xffffffff)
"#;
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(&path)
        .arg(PASSWORD_STR)
        .arg(DEFLATE_ENTRY)
        .arg(STORED_ENTRY)
        .arg(PRECOMP_ENTRY)
        .output()
        .expect("python3 binary not available");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "python zipfile verification failed:\nstdout:\n{stdout}\nstderr:\n{stderr}");

    let expected = [(DEFLATE_ENTRY, &deflate_data), (STORED_ENTRY, &stored_data), (PRECOMP_ENTRY, &precomp_data)];
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap();
        let len: usize = parts.next().unwrap().parse().unwrap();
        let crc: u32 = parts.next().unwrap().parse().unwrap();
        let (_, data) = expected.iter().find(|(n, _)| *n == name).unwrap();
        assert_eq!(len, data.len(), "length mismatch for {name}");
        assert_eq!(crc, crc32(data), "CRC mismatch for {name}");
    }
}
