# Migration
A migration guide for notable & breaking changes will be documented in this file.
All intermediate migration steps should be applied if migrating from multiple versions behind.

## [Unreleased]

- [`ZipOptions::validate_eor_is_eoa`] has been renamed to [`ZipOptions::validate_eoa_is_eor`].
  Its behaviour is unchanged; only the name was the wrong way around.
- [`CEOCDR::eocdr64`] and [`CEOCDR::eocdl64`] have been renamed to [`CEOCDR::eocdr64h`] and
  [`CEOCDR::eocdl64h`] to match the header types they hold. Both are still `Option`s, and
  [`CEOCDR::is_zip64()`] remains the way to test for a ZIP64 archive.
- [`spec::headers1::EOCDR64H`]'s [`KnownSize::SIZE`] is now 52 rather than 56. If you were
  relying on that constant to size your own reads, you now need to account for the 4-byte
  signature separately.
- [`base::read::stream::ZipFileReader`] is now deprecated in favour of
  [`base::read1::stream::ZipArchiveReader`], completing the deprecation of the old read module
  which began in 0.0.19. It still works as before. To move across ahead of the eventual removal:
  - The `Ready`/`Reading` type states have gone. A single [`ZipArchiveReader`] is borrowed
    mutably for each file rather than consumed and handed back, so there is no state to thread
    through your loop.
  - `new()` -> `new()` or `new_with_options()`. There is no `with_tokio()` equivalent; use
    `tokio_util::compat` to adapt a `tokio` reader to the `futures` IO traits.
  - `next_without_entry()` and `next_with_entry()` -> `next()`, which yields a borrowed
    [`base::read1::ZipFileReader`]. Its metadata comes from the local file ([`LF`]) via `lf()`
    rather than [`ZipEntry`], and the central directory is available from `cdrs()` once `next()`
    has returned `None`.
  - `reader()` and `reader_mut()` -> what `next()` hands you is the file reader itself.
  - `done()` -> nothing. Validation which it performed (EOF reached, CRC and sizes) is now
    configured through [`ZipOptions`] and applied automatically, and the next call to `next()`
    reclaims the reader.
  - `skip()` -> [`base::read1::ZipFileReader::skip()`], though calling `next()` drains whatever
    is left of the current file for you.
  - `into_inner()` has no equivalent; the archive reader does not give the source back.
  - Files written with a data descriptor are not yet supported by [`base::read1::stream`]. If you
    depend on those, stay on the deprecated reader until support lands.

## [Unreleased - read1 -> read migration]

- The [`base::read1`] module has been renamed to [`base::read`], replacing the old module which has
  now been removed. Apply the entries above, reading `read1` as `read`.
- The deprecated [`base::read::mem`] and [`tokio::read::fs`] modules have been removed.

## [0.0.19] - 2026-08-22

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
