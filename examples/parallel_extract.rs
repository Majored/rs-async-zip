// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::sync::Arc;

use tokio::fs::File;
use async_zip::read::concurrent::ZipFileReader;

#[tokio::main]
async fn main() {
    let zip = Arc::new(ZipFileReader::new("./Archive.zip").await.unwrap());
    let mut handles = Vec::with_capacity(zip.entries().len());

    for (index, _) in zip.entries().iter().enumerate() {
        let local_zip = zip.clone();

        handles.push(tokio::spawn(async move {
            let mut reader = local_zip.entry_reader(index).await.unwrap();
            let mut output = File::create(format!("./output/{}", reader.entry().name())).await.unwrap();

            tokio::io::copy(&mut reader, &mut output).await.unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}