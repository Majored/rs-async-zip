// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::path::Path;
use std::sync::Arc;

use async_zip::read::fs::ZipFileReader;
use tokio::fs::File;

// NOTE: This example does not check nor mitigate any potienal directory traversal. It expects a trusted ZIP file to be
//       provided. Mitigation should be added by the library implementer where needed.

#[tokio::main]
async fn main() {
    let zip = Arc::new(ZipFileReader::new("./Archive.zip").await.unwrap());
    let mut handles = Vec::with_capacity(zip.entries().len());

    for (index, entry) in zip.entries().iter().enumerate() {
        if entry.dir() {
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
