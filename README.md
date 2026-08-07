<div align="center">

# `region-rs`

## Cross-platform virtual memory API

[![GitHub CI Status][github-shield]][github]
[![crates.io version][crate-shield]][crate]
[![Documentation][docs-shield]][docs]
[![License][license-shield]][license]

</div>

This crate provides a cross-platform Rust API for allocating, querying and
manipulating virtual memory. It is a thin abstraction over platform APIs such as
`VirtualQuery`/`VirtualAlloc`/`VirtualLock` on Windows and
`mprotect`/`mmap`/`mlock` (and friends) on Unix-like systems.

## Platforms

Continuously tested against:

- Linux (`gnu` / `musl`)
- Windows (`gnu` / `msvc`)
- macOS
- FreeBSD
- OpenBSD
- NetBSD

Also checked / supported where practical:

- Android (compile-checked; full cross tests currently blocked by toolchain linking)
- Illumos
- GNU/Hurd (compile-checked via nightly `-Zbuild-std=core,alloc`; no prebuilt `rust-std`)
- Redox (allocation/protection; region querying is not yet available)

## Installation

```toml
[dependencies]
region = "4.0.0"
```

The default feature set includes `std`. Disable it to use the crate as
`#![no_std]` + `alloc`:

```toml
region = { version = "4.0.0", default-features = false }
```

## Example

```rust
use region::Protection;

# fn main() -> region::Result<()> {
let data = [0xDE, 0xAD, 0xBE, 0xEF];

// Page size
let pz = region::page::size();

// VirtualQuery | '/proc/self/maps'
let q  = region::query(data.as_ptr())?;
let qr = region::query_range(data.as_ptr(), data.len())?;

// VirtualAlloc | mmap
let alloc = region::alloc(100, Protection::READ_WRITE)?;

// VirtualProtect | mprotect
unsafe {
  region::protect(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
}

// Temporarily change one or more pages' protection
let handle = unsafe {
  region::protect_with_handle(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?
};

// VirtualLock | mlock
let guard = region::lock(data.as_ptr(), data.len())?;
# let _ = (pz, q, qr, alloc, handle, guard);
# Ok(())
# }
```

## Compatibility notes

- This is a major release (`4.0.0`): Rust edition is `2024`, MSRV is `1.85`, and
  a few APIs/error types changed for `no_std` friendliness and safer locking
  semantics.
- `bitflags` is now `2.x`. Prefer `Protection::from_bits_retain` over the
  deprecated `from_bits_unchecked`.
- `windows-sys` accepts `>=0.52, <=0.61` so dependents can unify versions.

<!-- Links -->
[github-shield]: https://img.shields.io/github/actions/workflow/status/darfink/region-rs/ci.yml?branch=master&label=actions&logo=github&style=for-the-badge
[github]: https://github.com/darfink/region-rs/actions/workflows/ci.yml?query=branch%3Amaster
[crate-shield]: https://img.shields.io/crates/v/region.svg?style=for-the-badge
[crate]: https://crates.io/crates/region
[docs-shield]: https://img.shields.io/badge/docs-crates-green.svg?style=for-the-badge
[docs]: https://docs.rs/region/
[license-shield]: https://img.shields.io/crates/l/region.svg?style=for-the-badge
[license]: https://github.com/darfink/region-rs
