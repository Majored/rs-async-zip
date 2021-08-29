# rs-async-zip
[![GitHub license](https://img.shields.io/badge/license-MIT-007ec6)](https://github.com/Majored/rs-async-zip/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/async_zip)](https://crates.io/crates/async_zip)

An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.

## Features
- Asynchronous design powered by `tokio`.
- Support for Stored, Deflate, Bzip2, LZMA, zstd, and xz compression methods.
- Aims for reasonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.

## Installation & Basic Usage

```toml
[dependencies]
async_zip = "0.0.1"
```

### Streaming
#### Reading
```rust
use async_zip::stream::read::ZipStreamReader;
use tokio::fs::File;
use tokio::io::BufReader;
...

let mut reader = BufReader::new(File::open("Test.zip").await.unwrap());
let mut zip = ZipStreamReader::new(&mut reader);

loop {
    let entry_opt = zip.next_entry().await.unwrap();

    if entry_opt.is_none() {
        break;
    }

    println!("Entry filename: {}", entry_opt.unwrap().file_name());
}
```

#### Writing
```rust
use async_zip::stream::write::ZipStreamWriter;
use async_zip::opts::ZipEntryOptions;
use async_zip::Compression;
use tokio::io::{BufWriter, AsyncWriteExt};
use tokio::fs::File;
...

let mut writer = BufWriter::new(File::create("Example.zip").await.unwrap());
let mut zip = ZipStreamWriter::new(&mut writer);

let mut guard = zip.new_entry(ZipEntryOptions::new("Example.txt".to_string(), Compression::Deflate)).await.unwrap();
guard.write(b"This is an example file.").await.unwrap();
guard.close().await.unwrap();

zip.close().await.unwrap();
```

## Contributions
Whilst I will be continuing to maintain this crate myself, reasonable specification compliance is a huge undertaking for a single individual. As such, contributions will always be encouraged.

## Issues & Support
