// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::spec::compression::Compression;
use crate::write::{EntryOptions, ZipFileWriter};

use std::io::Cursor;
use std::vec::Vec;

#[tokio::test]
async fn empty() {
    use crate::read::seek::ZipFileReader;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let zip_writer = ZipFileWriter::new(&mut input_stream);
    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let zip_reader = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");
    assert!(zip_reader.entries().is_empty());
}

#[tokio::test]
async fn zero_length_zip() {
    use crate::read::seek::ZipFileReader;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let zip_reader_res = ZipFileReader::new(&mut input_stream).await;
    assert!(zip_reader_res.is_err());
}

#[tokio::test]
async fn single_entry_no_data() {
    use crate::read::seek::ZipFileReader;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let mut zip_writer = ZipFileWriter::new(&mut input_stream);
    let open_opts = EntryOptions::new("foo.bar".to_string(), Compression::Stored);

    zip_writer.write_entry_whole(open_opts, &[]).await.expect("failed to write entry");
    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let zip_reader = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");

    assert_eq!(1, zip_reader.entries().len());
    assert!(zip_reader.entry("foo.bar").is_some());
    assert_eq!(0, zip_reader.entry("foo.bar").unwrap().0);
    assert_eq!(0, zip_reader.entry("foo.bar").unwrap().1.compressed_size().expect("no compressed size"));
    assert_eq!(0, zip_reader.entry("foo.bar").unwrap().1.uncompressed_size().expect("no uncompressed size"));
    assert_eq!(Compression::Stored, *zip_reader.entry("foo.bar").unwrap().1.compression());
}

macro_rules! single_entry_gen {
    ($name:ident, $typ:expr) => {
        #[tokio::test]
        async fn $name() {
            use crate::read::seek::ZipFileReader;

            let mut input_stream = Cursor::new(Vec::<u8>::new());

            let mut zip_writer = ZipFileWriter::new(&mut input_stream);
            let open_opts = EntryOptions::new("foo.bar".to_string(), $typ);
            let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt...";

            zip_writer.write_entry_whole(open_opts, data.as_bytes()).await.expect("failed to write entry");
            zip_writer.close().await.expect("failed to close writer");

            input_stream.set_position(0);

            let mut zip_reader = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");

            assert_eq!(1, zip_reader.entries().len());

            let entry = zip_reader.entry("foo.bar").expect("no 'foo.bar' entry");
            assert_eq!(0, entry.0);
            assert!(entry.1.compressed_size().is_some());
            assert_eq!(data.len() as u32, entry.1.uncompressed_size().expect("no uncompressed size"));
            assert_eq!($typ, *entry.1.compression());

            let entry_reader = zip_reader.entry_reader(0).await.expect("failed to open entry reader");
            let buffer = entry_reader.read_to_string_crc().await.expect("failed to read entry to string");

            assert_eq!(data, buffer);
        }
    };
}

single_entry_gen!(single_entry_stored, Compression::Stored);
single_entry_gen!(single_entry_deflate, Compression::Deflate);
single_entry_gen!(single_entry_bz, Compression::Bz);
single_entry_gen!(single_entry_lzma, Compression::Lzma);
single_entry_gen!(single_entry_zstd, Compression::Zstd);
single_entry_gen!(single_entry_xz, Compression::Xz);
