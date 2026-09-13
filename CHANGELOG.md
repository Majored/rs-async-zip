# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

### Added

- A streaming reader for the re-written read module at [`base::read1::stream::ZipArchiveReader`],
  which reads an archive front-to-back over any [`AsyncBufRead`] source without requiring
  [`AsyncSeek`]. It provides:
  - [`next()`], which yields a [`base::read1::ZipFileReader`] per local file. Returning `None`
    means the whole end of archive (every CDR, the ZIP64 EOCDR/EOCDL if present, the EOCDR and
    the archive comment) has been read and all configured validations have run.
  - [`lfs()`], [`cdrs()`] and [`ceocdr()`] accessors. The latter two are only populated once
    [`next()`] has returned `None`, as the central directory trails the local files.
  - [`raw_next_header()`] and [`raw_assume_lf()`], a lower-level pair for driving the stream
    header-by-header via [`base::read1::stream::ZipStreamHeader`].
  - Reading a file no longer needs to be finished before moving on; the reader drains whatever is
    left of the current file when the next one is requested.
  - This is still in active development. Files written with a data descriptor are not yet
    supported and currently fail with [`ZipError::FeatureNotSupported`].
- [`base::read1::ZipFileReader::skip()`], which reads and discards the remainder of a file.
- New [`ZipOptions`] fields:
  - [`stream_fully_consume_archive`], whether the streaming reader reads the end of the archive
    after the last local file. Disabling it also skips the validations which depend on it.
  - [`validate_cd_against_seen_when_streaming`], whether each CDR is validated against the local
    file header seen for it earlier in the stream.
- New [`ZipError`] variants for malformed streams: `MoreLFHsThanCDRs`,
  `MalformedOutOfOrderHeader` and `MalformedMissingHeader`.
- An `examples/validate.rs` example, which uses the seeking reader to check that every file in an
  archive reads without error.
- Further sample archives (nested, many files, project, mixed, 1MB) and tests which read every
  file within them, plus streaming variants of the existing empty and simple tests.

### Changed

- [`ZipOptions::validate_eor_is_eoa`] has been renamed to
  [`ZipOptions::validate_eoa_is_eor`], matching what it actually asserts.
- [`CEOCDR::eocdr64`] and [`CEOCDR::eocdl64`] have been renamed to
  [`CEOCDR::eocdr64h`] and [`CEOCDR::eocdl64h`], as they hold the header structures.
- [`spec::headers1::Signature`] now derives [`Hash`], and [`spec::headers1::LFH`] and
  [`spec::constructs::LF`] now derive [`Clone`].
- The [`base::read1`] module docs now compare the seeking and streaming readers accurately, now
  that both exist.

### Deprecated

### Removed

### Fixed

- [`spec::headers1::EOCDR64H`]'s [`KnownSize::SIZE`] was 56 rather than 52, as it incorrectly
  counted the 4-byte signature which is read separately.

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

[Unreleased]: https://github.com/Majored/rs-async-zip/compare/v0.0.19...HEAD
[0.0.19]: https://github.com/Majored/rs-async-zip/compare/v0.0.18...v0.0.19
