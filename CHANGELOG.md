# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://doc.rust-lang.org/cargo/reference/semver.html).

## [Unreleased]

### Added

- This changelog, MIGRATION.md and TERMINOLOGY.md.
- Tracing as an optional dependency, enabled via the `tracing` feature.
- An initial re-write of the read module. This is still in active development, with no stream
  reader currently. The intent is to fully replace the existing `read` module with it. Improvements
  include a large set of configurable validation checks. This reader is intended to be in feature
  parity with the old read module.

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

## [0.0.18] - 2025-08-09
- Start of this changelog.

[Unreleased]: https://github.com/Majored/rs-async-zip/compare/v0.0.18...HEAD
