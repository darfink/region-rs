#![deny(missing_docs)]
//! A library for manipulating memory regions
//!
//! This crate provides several functions for handling memory pages and regions.
//! It is implemented using platform specific APIs. The library exposes both low
//! and high level functionality for manipulating pages.
//!
//! Not all OS specific quirks are abstracted away. For instance; some OSs
//! enforce memory pages to be readable whilst other may prevent pages from
//! becoming executable (i.e DEP).
//!
//! *Note: a region is a collection of one or more pages laying consecutively in
//! memory, with the same properties.*
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
//!   let pc = region::page::ceil(1234);
//!   let pf = region::page::floor(1234);
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
extern crate libc;

pub use error::{Error, Result};
pub use lock::{lock, unlock, LockGuard};
pub use protect::{protect, protect_with_handle, ProtectGuard, Protection};

mod error;
mod lock;
mod os;
pub mod page;
mod protect;

/// A descriptor for a memory region
///
/// This type acts as a POD-type, i.e it has no functionality but merely
/// stores region information.
#[derive(Debug, Clone, Copy)]
pub struct Region {
  /// Base address of the region
  pub base: *const u8,
  /// Whether the region is guarded or not
  pub guarded: bool,
  /// Protection of the region
  pub protection: Protection,
  /// Whether the region is shared or not
  pub shared: bool,
  /// Size of the region (multiple of page size)
  pub size: usize,
}

impl Region {
  /// Returns the region's lower bound.
  pub fn lower(&self) -> usize {
    self.base as usize
  }

  /// Returns the region's upper bound.
  pub fn upper(&self) -> usize {
    self.lower() + self.size
  }
}

unsafe impl Send for Region {}
unsafe impl Sync for Region {}

/// Queries the OS with an address, returning the region it resides within.
///
/// The implementation uses `VirtualQuery` on Windows, `mach_vm_region` on macOS,
/// `kinfo_getvmmap` on FreeBSD, and parses `proc/[pid]/maps` on Linux.
///
/// - The enclosing region can be of multiple page sizes.
/// - The address is rounded down to the closest page boundary.
/// - The address may not be null.
///
/// # Examples
///
/// ```
/// use region::{Protection};
///
/// let data = [0; 100];
/// let region = region::query(data.as_ptr()).unwrap();
///
/// assert_eq!(region.protection, Protection::READ_WRITE);
/// ```
pub fn query(address: *const u8) -> Result<Region> {
  if address.is_null() {
    return Err(Error::NullAddress);
  }

  // The address must be aligned to the closest page boundary
  os::get_region(page::floor(address as usize) as *const u8)
}

/// Queries the OS with a range, returning the regions it contains.
///
/// A 2-byte range straddling a page boundary will return both pages (or one
/// region, if the pages have the same properties). The implementation uses
/// `query` internally.
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The address may not be null.
/// - The size may not be zero.
///
/// # Examples
///
/// ```
/// let data = [0; 100];
/// let region = region::query_range(data.as_ptr(), data.len()).unwrap();
///
/// assert!(region.len() > 0);
/// ```
pub fn query_range(address: *const u8, size: usize) -> Result<Vec<Region>> {
  if size == 0 {
    return Err(Error::EmptyRange);
  }

  let mut result = Vec::new();
  let mut base = page::floor(address as usize);
  let limit = address as usize + size;

  loop {
    let region = query(base as *const u8)?;
    result.push(region);
    base = region.upper();

    if limit <= region.upper() {
      break;
    }
  }

  Ok(result)
}

#[cfg(test)]
mod tests {
  extern crate memmap;

  use self::memmap::MmapMut;
  use super::*;

  pub fn alloc_pages(prots: &[Protection]) -> MmapMut {
    let pz = page::size();
    let map = MmapMut::map_anon(pz * prots.len()).unwrap();
    let mut base = map.as_ptr();

    for protection in prots {
      unsafe {
        protect(base, pz, *protection).unwrap();
        base = base.offset(pz as isize);
      }
    }

    map
  }

  #[test]
  fn query_null() {
    assert!(query(::std::ptr::null()).is_err());
  }

  #[test]
  fn query_code() {
    let region = query(query_code as *const () as *const u8).unwrap();

    assert_eq!(region.guarded, false);
    assert_eq!(region.shared, cfg!(windows));
  }

  #[test]
  fn query_alloc() {
    let size = page::size() * 2;
    let mut map = alloc_pages(&[Protection::READ_EXECUTE, Protection::READ_EXECUTE]);
    let region = query(map.as_ptr()).unwrap();

    assert_eq!(region.guarded, false);
    assert_eq!(region.protection, Protection::READ_EXECUTE);
    assert!(!region.base.is_null() && region.base <= map.as_mut_ptr());
    assert!(region.size >= size);
  }

  #[test]
  fn query_area_zero() {
    assert!(query_range(&query_area_zero as *const _ as *const u8, 0).is_err());
  }

  #[test]
  fn query_area_overlap() {
    let pz = page::size();
    let prots = [Protection::READ_EXECUTE, Protection::READ_WRITE];
    let map = alloc_pages(&prots);

    // Query an area that overlaps both pages
    let address = unsafe { map.as_ptr().offset(pz as isize - 1) };
    let result = query_range(address, 2).unwrap();

    assert_eq!(result.len(), prots.len());
    for i in 0..prots.len() {
      assert_eq!(result[i].protection, prots[i]);
    }
  }

  #[test]
  fn query_area_alloc() {
    let pz = page::size();
    let prots = [
      Protection::READ,
      Protection::READ_WRITE,
      Protection::READ_EXECUTE,
    ];
    let map = alloc_pages(&prots);

    // Confirm only one page is retrieved
    let result = query_range(map.as_ptr(), pz).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].protection, prots[0]);

    // Retrieve all allocated pages
    let result = query_range(map.as_ptr(), pz * prots.len()).unwrap();
    assert_eq!(result.len(), prots.len());
    assert_eq!(result[1].size, pz);
    for i in 0..prots.len() {
      assert_eq!(result[i].protection, prots[i]);
    }
  }
}
