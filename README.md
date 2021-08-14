# rs-async-zip
[![GitHub license](https://img.shields.io/badge/license-MIT-007ec6)](https://github.com/Majored/mcm-js-api-wrapper/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/async_zip)](https://crates.io/crates/async_zip)

An asynchronous ZIP archive reading/writing crate with a heavy focus on streaming support.

## Features
- Asynchronous design powered by `tokio`.
- Support for Stored, Deflate, Bzip2, LZMA, zstd, and xz compression methods.
- Aims for reasonable [specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) compliance.

## Installation & Basic Usage

```toml
[dependencies]
async_zip = 0.0.1
```
## Contributions
Whilst I will be continuing to maintain this crate myself, reasonable specification compliance is a huge undertaking for a single individual. As such, contributions will always be encouraged.

## Issues & Support