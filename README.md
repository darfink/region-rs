region
======
[![Azure build Status][azure-shield]](azure)
[![Cirrus build status][cirrus-shield]][cirrus]
[![crates.io version][crate-shield]][crate]
[![Documentation][docs-shield]][docs]
[![Language (Rust)][rust-shield]][rust]

A Rust library for dealing with memory regions.

It is implemented using platform specific APIs (e.g `VirtualQuery`,
`VirtualLock`, `mprotect`, `mlock`).

## Platforms

This library has (so far) support for:
- Windows
- Linux & Android
- macOS & iOS
- FreeBSD

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
region = "2.1.0"
```

and this to your crate root:

```rust
extern crate region;
```

## Example

- Cross-platform equivalents:
```rust
let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];

// Page size
let pz = region::page::size();
let pc = region::page::ceil(1234);
let pf = region::page::floor(1234);

// VirtualQuery | '/proc/self/maps'
let q  = region::query(ret5.as_ptr())?;
let qr = region::query_range(ret5.as_ptr(), ret5.len())?;

// VirtualProtect | mprotect
region::protect(ret5.as_ptr(), ret5.len(), Protection::ReadWriteExecute)?;

// ... you can also temporarily change a region's protection
let handle = region::protect_with_handle(ret5.as_ptr(), ret5.len(), Protection::ReadWriteExecute)?;

// VirtualLock | mlock
let guard = region::lock(ret5.as_ptr(), ret5.len())?;
```

<!-- Links -->
[azure-shield]: https://dev.azure.com/darfink/region-rs/_apis/build/status/darfink.region-rs?branchName=master
[azure]: https://dev.azure.com/darfink/region-rs/_build/latest?definitionId=1&branchName=master
[cirrus-shield]: https://api.cirrus-ci.com/github/darfink/region-rs.svg
[cirrus]: https://cirrus-ci.com/github/darfink/region-rs
[crate-shield]: https://img.shields.io/crates/v/region.svg?style=flat-square
[crate]: https://crates.io/crates/region
[rust-shield]: https://img.shields.io/badge/powered%20by-rust-blue.svg?style=flat-square
[rust]: https://www.rust-lang.org
[docs-shield]: https://img.shields.io/badge/docs-crates-green.svg?style=flat-square
[docs]: https://docs.rs/region/