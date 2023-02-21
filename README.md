# async_zip
[![Crates.io](https://img.shields.io/crates/v/async_zip?style=flat-square)](https://crates.io/crates/async_zip)
[![Crates.io](https://img.shields.io/crates/d/async_zip?style=flat-square)](https://crates.io/crates/async_zip)
[![docs.rs](https://img.shields.io/docsrs/async_zip?style=flat-square)](https://docs.rs/async_zip/)
[![GitHub Workflow Status (branch)](https://img.shields.io/github/actions/workflow/status/Majored/rs-async-zip/ci-linux.yml?branch=main&style=flat-square)](https://github.com/Majored/rs-async-zip/actions?query=branch%3Amain)
[![GitHub](https://img.shields.io/github/license/Majored/rs-async-zip?style=flat-square)](https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

An asynchronous ZIP archive reading/writing crate powered by [`tokio`](https://crates.io/crates/tokio).

## Features
- Support for Stored, Deflate, bzip2, LZMA, zstd, and xz compression methods.
- Various different reading approaches (seek, stream, filesystem, in-memory buffer, etc).
- Support for writing complete data (u8 slices) or streams using data descriptors.
- Initial support for ZIP64 reading and writing.
- Aims for reasonable [specification](https://github.com/Majored/rs-async-zip/blob/main/SPECIFICATION.md) compliance.

## Installation & Basic Usage

```toml
[dependencies]
async_zip = { version = "0.0.12", features = ["full"] }
```

A (soon to be) extensive list of [examples](https://github.com/Majored/rs-async-zip/tree/main/examples) can be found under the `/examples` directory.

### Feature Flags
- `full` - Enables all below features.
- `chrono` - Enables support for parsing dates via `chrono`.
- `fs` - Enables support for the `fs` reading module.
- `deflate` - Enables support for the Deflate compression method.
- `bzip2` - Enables support for the bzip2 compression method.
- `lzma` - Enables support for the LZMA compression method.
- `zstd` - Enables support for the zstd compression method.
- `xz` - Enables support for the xz compression method.

### Reading
```rust
use tokio::{io::AsyncReadExt, fs::File};
use async_zip::read::seek::ZipFileReader;
...

let mut file = File::open("./Archive.zip").await.unwrap();
let mut zip = ZipFileReader::new(&mut file).await.unwrap();

let entry = zip.file().entries().get(0).unwrap().clone();
let mut string = String::new();
let mut reader = zip.entry(0).await.unwrap();
let txt = reader.read_to_string_checked(&mut string, entry.entry()).await.unwrap();

println!("{}", txt);
```

### Writing
```rust
use async_zip::write::ZipFileWriter;
use async_zip::{Compression, ZipEntryBuilder};
use tokio::fs::File;
...

let mut file = File::create("foo.zip").await.unwrap();
let mut writer = ZipFileWriter::new(&mut file);

let data = b"This is an example file.";
let builder = ZipEntryBuilder::new(String::from("bar.txt"), Compression::Deflate);

writer.write_entry_whole(builder, data).await.unwrap();
writer.close().await.unwrap();
```

## Contributions
Whilst I will be continuing to maintain this crate myself, reasonable specification compliance is a huge undertaking for a single individual. As such, contributions will always be encouraged and appreciated.

No contribution guidelines exist but additions should be developed with readability in mind, with appropriate comments, and make use of `rustfmt`.

## Issues & Support
Whether you're wanting to report a bug you've come across during use of this crate or are seeking general help/assistance, please utilise the [issues tracker](https://github.com/Majored/rs-async-zip/issues) and provide as much detail as possible (eg. recreation steps).

I try to respond to issues within a reasonable timeframe.
