region
======
[![Azure build Status][azure-shield]][azure]
[![Cirrus build status][cirrus-shield]][cirrus]
[![crates.io version][crate-shield]][crate]
[![Documentation][docs-shield]][docs]
[![Language (Rust)][rust-shield]][rust]

A Rust library for dealing with memory regions.

It is implemented using platform specific APIs (e.g `VirtualQuery`,
`VirtualLock`, `mprotect`, `mlock`).

## Platforms

This library provides CI for the following targets:

- Linux
  * `aarch64-linux-android`
  * `armv7-unknown-linux-gnueabihf`
  * `i686-unknown-linux-gnu`
  * `x86_64-unknown-linux-gnu`
- Windows
  * `x86_64-pc-windows-gnu`
  * `x86_64-pc-windows-msvc`
- macOS
  * `x86_64-apple-darwin`
- FreeBSD
  * `x86_64-unknown-freebsd`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
region = "2.2.0"
```

and this to your crate root:

```rust
extern crate region;
```

## Example

- Cross-platform equivalents:
```rust
let data = [0xDE, 0xAD, 0xBE, 0xEF];

// Page size
let pz = region::page::size();
let pc = region::page::ceil(1234);
let pf = region::page::floor(1234);

// VirtualQuery | '/proc/self/maps'
let q  = region::query(data.as_ptr())?;
let qr = region::query_range(data.as_ptr(), data.len())?;

// VirtualProtect | mprotect
region::protect(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;

// ... you can also temporarily change a region's protection
let handle = region::protect_with_handle(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;

// VirtualLock | mlock
let guard = region::lock(data.as_ptr(), data.len())?;
```

<!-- Links -->
[azure-shield]: https://img.shields.io/azure-devops/build/darfink/region-rs/1/master?label=Azure%20Pipelines&logo=azure-pipelines&style=flat-square
[azure]: https://dev.azure.com/darfink/region-rs/_build/latest?definitionId=1&branchName=master
[cirrus-shield]: https://img.shields.io/cirrus/github/darfink/region-rs/master?label=FreeBSD&logo=cirrus-ci&style=flat-square
[cirrus]: https://cirrus-ci.com/github/darfink/region-rs
[crate-shield]: https://img.shields.io/crates/v/region.svg?style=flat-square
[crate]: https://crates.io/crates/region
[rust-shield]: https://img.shields.io/badge/powered%20by-rust-blue.svg?style=flat-square
[rust]: https://www.rust-lang.org
[docs-shield]: https://img.shields.io/badge/docs-crates-green.svg?style=flat-square
[docs]: https://docs.rs/region/
