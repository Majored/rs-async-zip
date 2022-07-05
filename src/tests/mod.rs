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
async fn single_entry_with_comment() {
    use crate::read::seek::ZipFileReader;

    let comment = "Foo bar.";
    let mut input_stream = Cursor::new(Vec::<u8>::new());
    let mut zip_writer = ZipFileWriter::new(&mut input_stream);

    zip_writer.comment(String::from(comment));
    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let zip_reader_res = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");

    assert!(zip_reader_res.comment().is_some());
    assert_eq!(zip_reader_res.comment().unwrap(), comment);
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

#[cfg(feature = "deflate")]
#[tokio::test]
async fn data_descriptor_single() {
    use crate::read::seek::ZipFileReader;
    use tokio::io::AsyncWriteExt;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let mut zip_writer = ZipFileWriter::new(&mut input_stream);
    let open_opts = EntryOptions::new("foo.bar".to_string(), Compression::Deflate);

    let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt...";
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");

    entry_writer.close().await.expect("failed to close entry");
    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let mut zip_reader = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");

    assert_eq!(1, zip_reader.entries().len());

    let entry = zip_reader.entry("foo.bar").expect("no 'foo.bar' entry");
    assert_eq!(0, entry.0);
    assert!(entry.1.compressed_size().is_some());
    assert!(entry.1.data_descriptor);
    assert_eq!(data.len() as u32, entry.1.uncompressed_size().expect("no uncompressed size"));
    assert_eq!(Compression::Deflate, *entry.1.compression());

    let entry_reader = zip_reader.entry_reader(0).await.expect("failed to open entry reader");
    let buffer = entry_reader.read_to_string_crc().await.expect("failed to read entry to string");

    assert_eq!(data, buffer);
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn data_descriptor_double_stream() {
    use crate::read::stream::ZipFileReader;
    use tokio::io::AsyncWriteExt;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let mut zip_writer = ZipFileWriter::new(&mut input_stream);
    let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt...";

    let open_opts = EntryOptions::new("foo.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    let open_opts = EntryOptions::new("test.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let mut zip_reader = ZipFileReader::new(&mut input_stream);

    assert!(!zip_reader.finished());
    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_some());
    let entry_reader = entry_reader.unwrap();

    let buffer = entry_reader.read_to_string_crc().await.expect("failed to read entry to string");
    assert_eq!(data, buffer);

    assert!(!zip_reader.finished());
    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_some());
    let entry_reader = entry_reader.unwrap();

    let buffer = entry_reader.read_to_string_crc().await.expect("failed to read entry to string");
    assert_eq!(data, buffer);

    assert!(!zip_reader.finished());

    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_none());
    assert!(zip_reader.finished());
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn data_tokio_copy_stream() {
    use crate::read::stream::ZipFileReader;
    use tokio::io::AsyncWriteExt;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let mut zip_writer = ZipFileWriter::new(&mut input_stream);
    let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt...";

    let open_opts = EntryOptions::new("foo.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    let open_opts = EntryOptions::new("test.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let mut zip_reader = ZipFileReader::new(&mut input_stream);

    assert!(!zip_reader.finished());
    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_some());

    let mut entry_reader = entry_reader.unwrap();

    let mut output_stream = Cursor::new(Vec::<u8>::new());

    assert_ne!(tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy"), 0);
    assert_eq!(tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy"), 0);

    assert!(entry_reader.compare_crc());
    assert_eq!(data, String::from_utf8(output_stream.into_inner()).expect("failed to read entry to string"));

    assert!(!zip_reader.finished());
    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_some());
    let mut entry_reader = entry_reader.unwrap();

    let mut output_stream = Cursor::new(Vec::<u8>::new());
    tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy");

    assert!(entry_reader.compare_crc());
    assert_eq!(data, String::from_utf8(output_stream.into_inner()).expect("failed to read entry to string"));

    assert!(!zip_reader.finished());

    let entry_reader = zip_reader.entry_reader().await.expect("failed to open entry reader");
    assert!(entry_reader.is_none());
    assert!(zip_reader.finished());
}

#[cfg(feature = "deflate")]
#[tokio::test]
async fn data_tokio_copy_seek() {
    use crate::read::seek::ZipFileReader;
    use tokio::io::AsyncWriteExt;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let mut zip_writer = ZipFileWriter::new(&mut input_stream);
    let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt...";

    let open_opts = EntryOptions::new("foo.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    let open_opts = EntryOptions::new("test.bar".to_string(), Compression::Deflate);
    let mut entry_writer = zip_writer.write_entry_stream(open_opts).await.expect("failed to open write entry");
    entry_writer.write_all(data.as_bytes()).await.expect("failed to write entry");
    entry_writer.close().await.expect("failed to close entry");

    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let mut zip_reader = ZipFileReader::new(&mut input_stream).await.unwrap();

    assert_eq!(2, zip_reader.entries().len());

    let mut entry_reader = zip_reader.entry_reader(0).await.expect("failed to open entry reader");

    let mut output_stream = Cursor::new(Vec::<u8>::new());

    assert_ne!(tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy"), 0);
    assert_eq!(tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy"), 0);

    assert!(entry_reader.compare_crc());
    assert_eq!(data, String::from_utf8(output_stream.into_inner()).expect("failed to read entry to string"));

    let mut entry_reader = zip_reader.entry_reader(1).await.expect("failed to open entry reader");

    let mut output_stream = Cursor::new(Vec::<u8>::new());
    tokio::io::copy(&mut entry_reader, &mut output_stream).await.expect("failed to copy");

    assert!(entry_reader.compare_crc());
    assert_eq!(data, String::from_utf8(output_stream.into_inner()).expect("failed to read entry to string"));
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
#[cfg(feature = "deflate")]
single_entry_gen!(single_entry_deflate, Compression::Deflate);
#[cfg(feature = "bzip2")]
single_entry_gen!(single_entry_bz, Compression::Bz);
#[cfg(feature = "lzma")]
single_entry_gen!(single_entry_lzma, Compression::Lzma);
#[cfg(feature = "zstd")]
single_entry_gen!(single_entry_zstd, Compression::Zstd);
#[cfg(feature = "xz")]
single_entry_gen!(single_entry_xz, Compression::Xz);
