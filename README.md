# rs-async-zip
[![GitHub license](https://img.shields.io/badge/license-MIT-007ec6)](https://github.com/Majored/rs-async-zip/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/async_zip)](https://crates.io/crates/async_zip)
[![docs.rs](https://img.shields.io/docsrs/async_zip/0.0.2)](https://docs.rs/async_zip/0.0.2/async_zip/)

An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.

## Features
- Asynchronous design powered by `tokio`.
- Support for Stored, Deflate, bzip2, LZMA, zstd, and xz compression methods.
- Various different reading approaches (seek, stream, filesystem, in-memory buffer).
- Support for writing a complete data (u8 slices) or stream writing using data descriptors (unimplemented).
- Aims for reasonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.

## Installation & Basic Usage

```toml
[dependencies]
async_zip = "0.0.2"
```

A (soon to be) extensive list of [examples](https://github.com/Majored/rs-async-zip/tree/main/examples) can be found under the `/examples` directory.

### Reading
```rust
use tokio::{io::AsyncReadExt, fs::File};
use async_zip::read::seek::ZipFileReader;
...

let mut file = File::open("./Archive.zip").await.unwrap();
let mut zip = ZipFileReader::new(&mut file).await.unwrap();

let mut reader = zip.entry_reader(0).await.unwrap();
let mut txt = String::new();

reader.read_to_string(&mut txt).await.unwrap();
println!("{}", txt);
```

## Contributions
Whilst I will be continuing to maintain this crate myself, reasonable specification compliance is a huge undertaking for a single individual. As such, contributions will always be encouraged and appreciated.

No contribution guidelines exist but additions should be developed with readability in mind, with appropriate comments, and make use of `rustfmt`.
## Issues & Support
Whether you're wanting to report a bug you've come across during use of this crate or are seeking general help/assistance, please utilise the [issues tracker](https://github.com/Majored/rs-async-zip/issues) and tag your issue appropriately during creation.

I try to respond to issues within a reasonable timeframe.
