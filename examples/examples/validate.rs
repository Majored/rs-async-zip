// Copyright (c) 2026 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

//! Demonstrates using the seeking reader to validate a ZIP file is valid/not malformed.

use futures_lite::{AsyncBufRead, AsyncSeek};
use tokio::{fs::File, io::BufReader};
use tokio_util::compat::TokioAsyncReadCompatExt;

use async_zip::base::read1::seek::ZipArchiveReader;

#[tokio::main]
async fn main() {
    let file = File::open("Archive.zip").await.expect("Failed to open zip file");
    let file = BufReader::new(file).compat();

    let mut archive = match ZipArchiveReader::open(file).await {
        Ok(archive) => archive,
        Err(e) => {
            eprintln!("Failed to open zip archive: {}", e);
            return;
        }
    };

    println!("Opened zip archive with {} entries successfully.", archive.cdrs().len());
    println!("Validating files within...");
    println!();

    for index in 0..archive.cdrs().len() {
        validate_file(index, &mut archive).await;
    }

    println!();
    println!("Validation complete.");
}

async fn validate_file<R: AsyncBufRead + AsyncSeek + Unpin>(index: usize, archive: &mut ZipArchiveReader<R>) {
    let cdr = archive.cdrs().get(index).expect("Failed to get CDR").clone();
    let file_name = cdr.insecure_file_name.as_str().expect("Valid UTF-8 file name");
    let mut entry_reader = archive.file(index).await.expect("Failed to read ZipEntry");
    let mut sink = futures_lite::io::sink();

    match futures_lite::io::copy(&mut entry_reader, &mut sink).await {
        Ok(_) => println!("OK... {}", file_name),
        Err(e) => eprintln!("ERROR... {} ... {}", file_name, e),
    }
}
