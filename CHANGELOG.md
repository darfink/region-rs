# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [4.0.0] - 2026-08-07

### Added

- Added an explicit MSRV of Rust 1.85.
- Added `#![no_std]` support with an optional `std` feature (enabled by
  default). The crate always uses `core`/`alloc` instead of assuming the
  standard library.
- Added GNU/Hurd support via the Linux `/proc/self/maps` query backend. CI currently omits a Hurd check job because `rust-std` for the target is not available via rustup.
- Added Redox OS support for allocation / protection / locking. Region querying
  is not available there yet and returns `Error::UnmappedRegion`.
- Added `Region::max_protection()`.
- Added a Windows-only regression covering large `Protection::NONE` address-space
  reservations.

### Changed

- Bumped the crate edition from 2018 to 2024.
- Upgraded `bitflags` to 2.x.
- Widened the Windows dependency to `windows-sys = ">=0.52, <=0.61"`.
- Updated `libc` and retained `mach2`.
- Adopted strict-provenance pointer helpers throughout the crate.
- Made `unlock` `unsafe`.
- Changed `Error::SystemCall` to carry a raw OS error code (`i32`) instead of
  `std::io::Error`, so system-call failures remain representable without `std`.
- On Windows, `alloc(..., Protection::NONE)` now reserves address space without
  committing pages.
- Modernized GitHub Actions workflows:
  - moved Android to compile-check coverage while cross linking is broken
  - replaced unmaintained `actions-rs/*` usage
  - updated checkout / toolchain / Pages deploy actions
  - added an explicit MSRV CI job
  - refreshed runner / BSD matrix versions

### Removed

- Dropped the ancient MIPS CI target that required a pinned Rust 1.52 toolchain.

### Fixed

- Fixed Clippy / documentation lint noise that blocked modern toolchains.
- Fixed Windows system-info caching to avoid `static mut` on recent Rust.

## [3.0.2] - 2024-03-25

### Removed

- Removed explicit support for OpenBSD < 7.x (req libc locking)

### Fixed

- Dropped explicit versioning of `libc` version.

## [3.0.1] - 2024-03-06

### Added

- Added support for NetBSD.
- Added support for non-standard UNIX flags w/ `Protection::from_bits_*`.

### Changed

- Replaced deprecated `winapi` with `windows-sys`.
- Replaced deprecated `mach` with `mach2`.

### Fixed

- Fixed `query` to recursively query pages on macOS.
- Fixed `MAP_JIT` to be set when allocating (R)WX pages on macOS (aarch64).

## [3.0.0] - 2021-08-05

### Added

- Added support for OpenBSD.
- Added support for Illumos.
- Added support for memory allocations (`alloc` & `alloc_at`).
- Added `QueryIter` for lazily iterating regions.
- Added `inline` annotation where applicable.

### Changed

- Addresses are now defined as `*const T` (instead of `*const u8`).
- `Region` state has been internalized, now exposed via methods.
- `Error` enumerations have been altered.
- `query_iter` now returns an iterator.

### Removed

- Removed `page::size_from_range`.
- Removed deprecated functionality.

[unreleased]: https://github.com/darfink/region-rs/compare/v4.0.0...HEAD
[4.0.0]: https://github.com/darfink/region-rs/compare/v3.0.2...v4.0.0
[3.0.2]: https://github.com/darfink/region-rs/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/darfink/region-rs/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/darfink/region-rs/compare/v2.2.0...v3.0.0
