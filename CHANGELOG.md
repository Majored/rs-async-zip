# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [Unreleased - read1 -> read migration]

### Added

- The [`base::read`] module has been re-written resulting in breaking changes. See our MIGRATION.md.

### Changed

### Deprecated

### Removed

- The old [`base::read`] module has been removed.
- The re-write [`base::read1`] module has been moved into the [`base::read`] module.

### Fixed

### Security

## [0.0.19] - 2026-08-22

### Added

- This changelog and MIGRATION.md.
- Terminology and high-level ZIP concepts (archive, file, LF/LFH, CDR/CDRH, EOCDR, CEOCDR)
  documented in the [`base`] module, and used consistently across the new API.
- Tracing as an optional dependency, enabled via the `tracing` feature. Included in `full`.
- An initial re-write of the read module. This is still in active development, with no stream
  reader currently. The intent is to fully replace the existing `read` module with it. Improvements
  include a large set of configurable validation checks. This reader is intended to be in feature
  parity with the old read module. It lives at [`base::read1`] until then, and provides:
  - [`base::read1::seek::ZipArchiveReader`], which reads the central directory up-front and opens
    files by index without consuming the source reader ([`file()`], or [`file_oneshot()`]).
  - [`base::read1::seek::ZipArchiveFactory`], a user-supplied generator function which produces
    additional readers over the same archive for concurrent/parallel file reads.
  - [`base::read1::seek::ZipArchiveInner`], shareable archive metadata which can be reused across
    readers via [`new_with_inner()`] to avoid re-reading the central directory.
  - [`base::read1::ZipFileReader`], an [`AsyncRead`] over a single file which validates the CRC32
    and uncompressed size on EOF.
  - [`base::read1::ZipOptions`], a set of configurable validations and limits, plus
    [`ZipOptions::untrusted()`] as a conservative starting point for untrusted archives.
  - ZIP64 support, including ZIP64-aware accessors on [`LF`], [`CDR`] and [`CEOCDR`].
- The [`spec`] module is now public, exposing [`spec::headers1`] (primitive headers),
  [`spec::constructs`] (headers combined with their variable-length data), [`spec::extra`]
  (extra fields), [`spec::ZipString`] and the [`spec::KnownSize`] trait.
- New [`ZipError`] variants covering header/CDR mismatches, configured limits, invalid offsets,
  missing ZIP64 records, and binary parse failures.

### Changed

- The `full` feature now includes `tracing`.
- The specification link in the README and crate docs now points at PKWARE's APPNOTE.TXT.

### Deprecated

- [`base::read::seek::ZipFileReader`], [`base::read::mem::ZipFileReader`] and
  [`tokio::read::fs::ZipFileReader`] in favour of [`base::read1`].

### Removed

### Fixed

### Security

- The new reader bounds what an archive can make it allocate or read on your behalf. Extra field
  block sizes, extra field counts, central directory size and file counts, and per-file compressed
  and uncompressed sizes are all checked against configured limits before anything is allocated.
  File names remain your responsibility to sanitise.

## [0.0.18] - 2025-08-09

- Start of this changelog.

[Unreleased]: https://github.com/Majored/rs-async-zip/compare/v0.0.18...HEAD
