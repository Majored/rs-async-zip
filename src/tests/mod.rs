// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use crate::Compression;

use std::io::Cursor;
use std::vec::Vec;

#[tokio::test]
async fn empty() {
    use crate::read::seek::ZipFileReader;
    use crate::write::ZipFileWriter;

    let mut input_stream = Cursor::new(Vec::<u8>::new());

    let zip_writer = ZipFileWriter::new(&mut input_stream);
    zip_writer.close().await.expect("failed to close writer");

    input_stream.set_position(0);

    let zip_reader = ZipFileReader::new(&mut input_stream).await.expect("failed to open reader");
    assert!(zip_reader.entries().is_empty());
}

#[tokio::test]
async fn single_entry_no_data() {
    use crate::read::seek::ZipFileReader;
    use crate::write::{ZipFileWriter, EntryOptions};

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
