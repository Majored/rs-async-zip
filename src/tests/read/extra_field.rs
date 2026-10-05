// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use futures_lite::io::{BufReader, Cursor};

use crate::base::read1::seek::ZipArchiveReader;

/// A zip with one stored entry, `plain.txt`, whose central directory record
/// carries `extra` as its extra field.
fn zip_with_extra_field(extra: &[u8]) -> Vec<u8> {
    let (name, data) = (b"plain.txt", b"x");
    let header = |zip: &mut Vec<u8>, extra_len: u16| {
        // version needed 2.0, flags 0, stored, time 0, date 1980-01-01
        zip.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]);
        zip.extend_from_slice(&crc32fast::hash(data).to_le_bytes());
        // compressed size 1, uncompressed size 1, name length 9
        zip.extend_from_slice(&[1, 0, 0, 0, 1, 0, 0, 0, 9, 0]);
        zip.extend_from_slice(&extra_len.to_le_bytes());
    };
    let mut zip = b"PK\x03\x04".to_vec();
    header(&mut zip, 0);
    zip.extend_from_slice(name);
    zip.extend_from_slice(data);
    let cd_offset = zip.len();
    zip.extend_from_slice(b"PK\x01\x02\x14\x00");
    header(&mut zip, extra.len() as u16);
    // comment length, disk number, attributes, local header offset 0
    zip.extend_from_slice(&[0; 14]);
    zip.extend_from_slice(name);
    zip.extend_from_slice(extra);
    let cd_size = zip.len() - cd_offset;
    zip.extend_from_slice(b"PK\x05\x06\x00\x00\x00\x00\x01\x00\x01\x00");
    zip.extend_from_slice(&(cd_size as u32).to_le_bytes());
    zip.extend_from_slice(&(cd_offset as u32).to_le_bytes());
    zip.extend_from_slice(&[0, 0]);
    zip
}

async fn names(extra: &[u8]) -> Vec<String> {
    let zip = zip_with_extra_field(extra);
    let reader = BufReader::new(Cursor::new(zip));
    let archive = ZipArchiveReader::open(reader).await.expect("archive should open");
    archive.cdrs().iter().map(|cdr| cdr.insecure_file_name.as_str().expect("name is utf-8").to_owned()).collect()
}

#[tokio::test]
async fn read1_opens_archive_with_non_utf8_unicode_path_extra_field() {
    // Info-ZIP Unicode Path (0x7075): version 1, NameCRC32 0, then FF FE, which is not UTF-8.
    let extra = [0x75, 0x70, 7, 0, 1, 0, 0, 0, 0, 0xFF, 0xFE];
    assert_eq!(names(&extra).await, vec!["plain.txt".to_owned()]);
}

#[tokio::test]
async fn read1_opens_archive_with_trailing_extra_field_padding() {
    // extended timestamp (0x5455) with 5 bytes of data, then 2 bytes of padding
    let extra = [0x55, 0x54, 5, 0, 1, 0, 0, 0, 0, 0, 0];
    assert_eq!(names(&extra).await, vec!["plain.txt".to_owned()]);
}
