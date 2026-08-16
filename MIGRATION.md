# Migration
A migration guide for notable & breaking changes will be documented in this file.
All intermediate migration steps should be applied if migrating from multiple versions behind.

## [Unreleased]

## [Unreleased - read1 -> read migration]

- A clearer definition of terminology has been used across the API. See TERMINOLOGY.md.
- The re-write [`base::read1`] module has been removed and moved into the [`base::read`] module.
  - [`ZipFileReader`] has been replaced with [`ZipArchiveReader`].
  - [`ZipFileReader`] is now used to represent the reader of individual files within a ZIP archive.
  - `new()`, `from_raw_parts()` have been replaced with `open()`, `open_with_options()`, `new_with_inner()`.
  - `reader_without_entry()` and `reader_with_entry()` are now `file()`. Users should clone the file
    CDR structure if they need this information whilst holding onto a file reader.
- [`base::read::mem`] and [`tokio::read::fs`] module has been removed. These both are specific
  implementations of a factory/generator approach. A new [`ZipArchiveFactory`] type has been
  introduced to achieve the same functionality through a user-defined generator function. 
-