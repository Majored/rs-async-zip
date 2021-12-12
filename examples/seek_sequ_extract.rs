// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

use std::path::Path;

use async_zip::read::seek::ZipFileReader;
use tokio::fs::File;

// NOTE: This example does not check nor mitigate any potienal directory traversal. It expects a trusted ZIP file to be
//       provided. Mitigation should be added by the library implementer where needed.

#[tokio::main]
async fn main() {
    let mut file = File::open("./Archive.zip").await.unwrap();
    let mut zip = ZipFileReader::new(&mut file).await.unwrap();

    for i in 0..zip.entries().len() {
        let reader = zip.entry_reader(i).await.unwrap();

        if reader.entry().dir() {
            continue;
        }

        let path_str = format!("./output/{}", reader.entry().name());
        let path = Path::new(&path_str);
        tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();

        let mut output = File::create(path).await.unwrap();
        reader.copy_to_end_crc(&mut output, 65536).await.unwrap();
    }
}
