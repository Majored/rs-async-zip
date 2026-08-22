# Migration
A migration guide for notable & breaking changes will be documented in this file.
All intermediate migration steps should be applied if migrating from multiple versions behind.

## [Unreleased]

- A clearer definition of terminology has been used across the new API. See the [`base`] module docs.
- The [`spec`] module is now public. Its contents were previously crate-private, so nothing has
  been removed from the public API.
- The [`base::read`] seek, mem, and [`tokio::read::fs`] readers are now deprecated in favour of
  [`base::read1`]. They still work as before, and [`base::read::stream`] is unaffected. To move
  across ahead of the eventual removal:
  - [`base::read::seek::ZipFileReader`] -> [`base::read1::seek::ZipArchiveReader`].
    [`ZipFileReader`] now names the reader of an individual file within an archive.
  - `new()` and `from_raw_parts()` -> `open()`, `open_with_options()`, `new_with_inner()`.
  - `reader_without_entry()` and `reader_with_entry()` -> `file()`. Clone the file's CDR if you
    need this information whilst holding onto a file reader.
  - `entries()` -> `cdrs()`, and file name lookups are done with `find()`, which returns an
    iterator as the specification permits duplicate file names.
  - Entry metadata now comes from the spec structures directly ([`CDR`], [`LF`]) rather than
    [`ZipEntry`]/[`StoredZipEntry`]. Sizes and offsets are read through their ZIP64-aware
    accessors, which are fallible.
  - Validation which was previously explicit (such as `read_to_string_checked()`) is now
    configured through [`ZipOptions`] and applied automatically on EOF.
  - [`base::read::mem::ZipFileReader`] and [`tokio::read::fs::ZipFileReader`] ->
    [`base::read1::seek::ZipArchiveFactory`], constructed via
    [`ZipArchiveReader::into_factory()`] with your own generator function.
  - There is no `tokio` equivalent of [`base::read1`]. Use `tokio_util::compat` to adapt a `tokio`
    reader to the `futures` IO traits.
  - See the [`base::read1::seek`] module docs for the full set of usage examples.

## [Unreleased - read1 -> read migration]

- The [`base::read1`] module has been renamed to [`base::read`], replacing the old module which has
  now been removed. Apply the entries above, reading `read1` as `read`.
- The deprecated [`base::read::mem`] and [`tokio::read::fs`] modules have been removed.
