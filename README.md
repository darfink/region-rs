region
======
[![Travis build status][travis-shield]][travis]
[![Appveyor build status][appveyor-shield]][appveyor]
[![crates.io version][crate-shield]][crate]
[![Language (Rust)][rust-shield]][rust]

A Rust library for dealing with memory regions.

It is implemented using platform specific APIs (e.g `VirtualQuery`,
`VirtualLock`, `mprotect`, `mlock`).

## Documentation

https://docs.rs/region

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
region = "0.0.6"
```

and this to your crate root:

```rust
extern crate region;
```

## Platforms

This library has (so far) support for `Windows`, `Linux` & `macOS`.

<!-- Links -->
[travis-shield]: https://img.shields.io/travis/darfink/region-rs.svg?style=flat-square
[travis]: https://travis-ci.org/darfink/region-rs
[appveyor-shield]: https://img.shields.io/appveyor/ci/darfink/region-rs/master.svg?style=flat-square
[appveyor]: https://ci.appveyor.com/project/darfink/region-rs
[crate-shield]: https://img.shields.io/crates/v/region.svg?style=flat-square
[crate]: https://crates.io/crates/region
[rust-shield]: https://img.shields.io/badge/powered%20by-rust-blue.svg?style=flat-square
[rust]: https://www.rust-lang.org
