<div align="center">

# `region-rs`

## Cross-platform virtual memory API for Rust

[![GitHub CI Status][github-shield]][github]
[![crates.io version][crate-shield]][crate]
[![Documentation][docs-shield]][docs]
[![License][license-shield]][license]

</div>

`region` is a cross-platform Rust API for allocating, querying, locking, and modifying the protection of virtual memory pages. It wraps native OS primitives—such as `VirtualAlloc`, `VirtualQuery`, `VirtualProtect`, and `VirtualLock` on Windows, and `mmap`, `mprotect`, `mlock`, and platform map queries on Unix-like systems—behind one small, `#![no_std]`-compatible interface.

## Platform support

| Tier | Platforms | Notes |
| --- | --- | --- |
| Tested in CI | Linux (`gnu`, `musl`), Windows (`msvc`, `gnu`), macOS, FreeBSD, OpenBSD, NetBSD | Full test suite on every push |
| Compile-checked | iOS, Android, illumos, GNU/Hurd, Redox | Redox supports allocation, protection, and locking; region querying returns `Error::UnmappedRegion`. GNU/Hurd is checked with `-Zbuild-std=core,alloc`. |

## Installation

Add `region` to your `Cargo.toml`:

```toml
[dependencies]
region = "4"
```

The `std` feature is enabled by default (adding `std::error::Error` and `std::io::Error` conversions). Disable default features for `#![no_std]` + `alloc`:

```toml
[dependencies]
region = { version = "4", default-features = false }
```

## Example

```rust
use region::Protection;

fn main() -> region::Result<()> {
  let data = [0xDE, 0xAD, 0xBE, 0xEF];

  // Query the OS page size
  let page_size = region::page::size();

  // Query mapped memory (VirtualQuery / procfs / mach_vm_region / kinfo)
  let region = region::query(data.as_ptr())?;
  let regions = region::query_range(data.as_ptr(), data.len())?;

  // Allocate anonymous pages (VirtualAlloc / mmap)
  let alloc = region::alloc(100, Protection::READ_WRITE)?;

  // Change page protection (VirtualProtect / mprotect)
  unsafe {
    region::protect(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
  }

  // Temporarily change protection; restored when `_guard` is dropped
  let _guard = unsafe {
    region::protect_with_handle(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?
  };

  // Lock pages in RAM (VirtualLock / mlock); unlocked when `_lock` is dropped
  let _lock = region::lock(data.as_ptr(), data.len())?;

  let _ = (page_size, region, regions, alloc);
  Ok(())
}
```

## Compatibility notes

- **MSRV & edition:** Rust 1.85+, edition 2024.
- **`bitflags` 2.x:** Prefer the safe `Protection::from_bits_retain` over the deprecated `from_bits_unchecked`.
- **Dependency ranges:** `windows-sys` accepts `>=0.52, <=0.61` and `mach2` accepts `>=0.4.2, <0.8` so downstream crates can unify on one version.
- **Upgrading from 3.x:** `unlock` is now `unsafe`, `Error::SystemCall` stores a raw `i32` OS error code (with `as_io_error` / `into_io_error` under `std`), and allocating with `Protection::NONE` on Windows reserves address space without committing physical pages. See [CHANGELOG.md](CHANGELOG.md) for full details.

## License

MIT. See [LICENSE](LICENSE).

<!-- Links -->
[github-shield]: https://img.shields.io/github/actions/workflow/status/darfink/region-rs/ci.yml?branch=master&label=actions&logo=github&style=for-the-badge
[github]: https://github.com/darfink/region-rs/actions/workflows/ci.yml?query=branch%3Amaster
[crate-shield]: https://img.shields.io/crates/v/region.svg?style=for-the-badge
[crate]: https://crates.io/crates/region
[docs-shield]: https://img.shields.io/badge/docs-crates-green.svg?style=for-the-badge
[docs]: https://docs.rs/region/
[license-shield]: https://img.shields.io/crates/l/region.svg?style=for-the-badge
[license]: LICENSE
