// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use async_zip::write::{ZipFileWriter, EntryOptions};
use async_zip::Compression;

use anyhow::{Result, bail, anyhow};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
        eprintln!("Usage: cli_compress <input file or directory> <output ZIP file name - defaults to 'Archive.zip'>");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let input_str = args.next().ok_or(anyhow!("No input file or directory specified."))?;
    let input_path = Path::new(&input_str);

    let output_str = args.next().unwrap_or("./Archive.zip".to_string());
    let output_path = Path::new(&output_str);

    if output_path.exists() {
        bail!("The output file specified already exists.");
    }
    if !input_path.exists() {
        bail!("The input file or directory specified doesn't exist.");
    }

    let mut output_file = File::create(output_path).await?;
    let mut output_writer = ZipFileWriter::new(&mut output_file);
    let mut count = 0;

    if input_path.is_dir() {
        let mut dir_stream = tokio::fs::read_dir(input_path).await?;

        while let Some(entry) = dir_stream.next_entry().await? {
            // TODO: Negate irrelevant path segments. The path of the input is mirrored into the ZIP file name
            // currently, meaning entering './' will prefix all entry file names with './'.
            write_entry(entry.path().as_path(), &output_path, &mut output_writer).await?;
            count += 1;
        }
    } else {
        write_entry(input_path,&output_path, &mut output_writer).await?;
        count += 1;
    }

    output_writer.close().await?;
    output_file.shutdown().await?;

    println!("Successfully written {} ZIP entry(ies) to '{}'.", count, output_path.display());

    Ok(())
}

async fn write_entry(entry_path: &Path, output_path: &Path, writer: &mut ZipFileWriter<'_, File>) -> Result<()> {
    let mut input_file = File::open(entry_path).await?;

    let filename = entry_path.file_name().ok_or(anyhow!("No input file name."))?;
    let filename = filename.to_str().ok_or(anyhow!("Input filename is not Unicode."))?;

    let mut path = output_path.to_path_buf();
    path.push(entry_path.parent().ok_or(anyhow!("No parent file."))?);
    path.set_file_name(filename);

    let filename = path.to_str().ok_or(anyhow!("Input filename is not Unicode."))?.to_owned();

    let entry_options = EntryOptions::new(filename, Compression::Deflate);
    let mut entry_writer = writer.write_entry_stream(entry_options).await?;

    // TODO: Use buffered reader and set capacity to 65536.
    tokio::io::copy(&mut input_file, &mut entry_writer).await?;
    entry_writer.close().await?;

    Ok(())
}