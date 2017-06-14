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
region = "0.0.7"
```

and this to your crate root:

```rust
extern crate region;
```

## Example

```rust
// Assembly (x86) for returning an integer (5)
let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
let mut view = View::new(ret5.as_ptr(), ret5.len()).unwrap();

view.exec_with_prot(Protection::ReadWriteExecute, || {
    // Within this closure the memory is read, write & executable
    let x: extern "C" fn() -> i32 = unsafe { std::mem::transmute(ret5.as_ptr()) };
    assert_eq!(x(), 5);
}).unwrap();

// The protection flags have been restored
assert_eq!(view.get_prot(), Some(Protection::Read));
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
