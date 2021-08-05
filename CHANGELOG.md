# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added 

- Added support for OpenBSD.
- Added support for Illumos.
- Added support for memory allocations (`alloc` & `alloc_at`).
- Added `QueryIter` for lazily iterating regions.

### Changed

- Internalized `Region` state, now exposed via methods.
- `query_iter` now returns an iterator.
- `Error` enumerations altered.

### Removed

- Removed `page::size_from_range`.
- Removed deprecated functionality.

[unreleased]: https://github.com/darfink/ItemAutocomplete/compare/v2.2.0...HEAD
