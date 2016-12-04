region
======

A Rust library for dealing with memory regions.

It is implemented using platform specific APIs (e.g `VirtualQuery`,
`VirtualLock`, `mprotect`, `mlock`).

## Documentation

https://docs.rs/region

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
region = "0.0.5"
```

and this to your crate root:

```rust
extern crate region;
```

## Platforms

This library has (so far) support for `Windows`, `Linux` & `macOS`.
