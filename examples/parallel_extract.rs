// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::path::Path;
use std::sync::Arc;

use async_zip::read::fs::ZipFileReader;
use tokio::fs::File;

#[tokio::main]
async fn main() {
    let zip = Arc::new(ZipFileReader::new("./Archive.zip").await.unwrap());
    let mut handles = Vec::with_capacity(zip.entries().len());

    for (index, entry) in zip.entries().iter().enumerate() {
        if entry.name().ends_with("/") {
            continue;
        }

        let local_zip = zip.clone();
        handles.push(tokio::spawn(async move {
            let reader = local_zip.entry_reader(index).await.unwrap();

            let path_str = format!("./output/{}", reader.entry().name());
            let path = Path::new(&path_str);
            tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();

            let mut output = File::create(path).await.unwrap();
            reader.copy_to_end_crc(&mut output, 65536).await.unwrap();
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
