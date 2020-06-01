#![deny(missing_docs)]
//! A cross-platform Rust API for manipulating memory regions
//!
//! This crate provides several functions for querying, modifying and locking
//! memory regions and their pages.
//!
//! It is implemented using platform specific APIs, but not all OS specific
//! quirks are abstracted away. For instance; some OSs enforce memory pages to be
//! readable whilst other may prevent pages from becoming executable (i.e DEP).
//!
//! *Note: a region is a collection of one or more pages laying consecutively in
//! memory, with the same properties.*
//!
//! # Parallelism
//!
//! The properties of virtual memory pages can change at any time, unless all
//! threads that are unaccounted for are stopped. This affects all type of
//! operations, e.g. [query](query()), [protect](protect()) and [lock](lock()).
//!
//! # Installation
//!
//! This crate is [on crates.io](https://crates.io/crates/region) and can be
//! used by adding `region` to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! region = "2.2.0"
//! ```
//!
//! and this to your crate root:
//!
//! ```rust
//! extern crate region;
//! ```
//!
//! # Examples
//!
//! - Cross-platform equivalents.
//!
//!   ```rust
//!   # unsafe fn example() -> region::Result<()> {
//!   # use region::Protection;
//!   let data = [0xDE, 0xAD, 0xBE, 0xEF];
//!
//!   // Page size
//!   let pz = region::page::size();
//!   let pc = region::page::ceil(data.as_ptr());
//!   let pf = region::page::floor(data.as_ptr());
//!
//!   // VirtualQuery | '/proc/self/maps'
//!   let q  = region::query(data.as_ptr())?;
//!   let qr = region::query_range(data.as_ptr(), data.len())?;
//!
//!   // VirtualProtect | mprotect
//!   region::protect(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
//!
//!   // ... you can also temporarily change a region's protection
//!   let handle = region::protect_with_handle(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
//!
//!   // VirtualLock | mlock
//!   let guard = region::lock(data.as_ptr(), data.len())?;
//!   # Ok(())
//!   # }
//!   ```

#[macro_use]
extern crate bitflags;

pub use error::{Error, Result};
pub use lock::{lock, unlock, LockGuard};
pub use protect::{protect, protect_with_handle, ProtectGuard, Protection};
pub use query::{query, query_range};

mod error;
mod lock;
mod os;
pub mod page;
mod protect;
mod query;

/// A descriptor for a mapped memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
  /// Base address of the region
  base: *const (),
  /// Whether the region is guarded or not
  guarded: bool,
  /// Protection of the region
  protection: Protection,
  /// Whether the region is shared or not
  shared: bool,
  /// Size of the region (multiple of page size)
  size: usize,
}

impl Region {
  /// Returns a pointer to the region's base address.
  pub fn as_ptr<T>(&self) -> *const T {
    self.base as *const T
  }

  /// Returns a range representing the region's address space.
  pub fn as_range(&self) -> std::ops::Range<usize> {
    (self.base as usize)..(self.base as usize).saturating_add(self.size)
  }

  /// Returns whether the region is readable or not.
  pub fn is_readable(&self) -> bool {
    self.protection & Protection::READ == Protection::READ
  }

  /// Returns whether the region is writable or not.
  pub fn is_writable(&self) -> bool {
    self.protection & Protection::WRITE == Protection::WRITE
  }

  /// Returns whether the region is executable or not.
  pub fn is_executable(&self) -> bool {
    self.protection & Protection::EXECUTE == Protection::EXECUTE
  }

  /// Returns whether the region is guarded or not.
  pub fn is_guarded(&self) -> bool {
    self.guarded
  }

  /// Returns whether the region is shared or not.
  pub fn is_shared(&self) -> bool {
    self.shared
  }

  /// Returns the size of the region.
  pub fn len(&self) -> usize {
    self.size
  }

  /// Returns the protection flags of the region.
  pub fn protection(&self) -> Protection {
    self.protection
  }
}

unsafe impl Send for Region {}
unsafe impl Sync for Region {}

/// Validates & rounds an address-size pair to their respective page boundary.
fn round_to_page_boundaries<T>(address: *const T, size: usize) -> Result<(*const T, usize)> {
  if size == 0 {
    return Err(Error::InvalidParameter("size"));
  }

  let size = (address as usize % page::size()).saturating_add(size);
  let size = page::ceil(size as *const T) as usize;
  Ok((page::floor(address), size))
}

#[cfg(test)]
mod tests {
  use super::*;
  use memmap::MmapMut;

  /// Allocates one or more sequential pages for each protection flag.
  pub fn alloc_pages(pages: &[Protection]) -> MmapMut {
    let map = MmapMut::map_anon(page::size() * pages.len()).unwrap();
    let mut base = map.as_ptr();

    for protection in pages {
      unsafe {
        protect(base, page::size(), *protection).unwrap();
        base = base.offset(page::size() as isize);
      }
    }

    map
  }

  #[test]
  fn round_to_page_boundaries_works() -> Result<()> {
    let pz = page::size();
    let values = &[
      ((1, pz), (0, pz * 2)),
      ((0, pz - 1), (0, pz)),
      ((0, pz + 1), (0, pz * 2)),
      ((pz - 1, 1), (0, pz)),
      ((pz + 1, pz), (pz, pz * 2)),
      ((pz, pz), (pz, pz)),
    ];

    for ((before_address, before_size), (after_address, after_size)) in values {
      let (address, size) = round_to_page_boundaries(*before_address as *const (), *before_size)?;
      assert_eq!((address, size), (*after_address as *const (), *after_size));
    }
    Ok(())
  }
}
