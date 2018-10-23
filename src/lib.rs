#![recursion_limit = "1024"]
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
//! region = "0.3"
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
//!   let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
//!
//!   // Page size
//!   let pz = region::page::size();
//!
//!   // VirtualQuery | '/proc/self/maps'
//!   let q  = region::query(ret5.as_ptr())?;
//!   let qr = region::query_range(ret5.as_ptr(), ret5.len())?;
//!
//!   // VirtualProtect | mprotect
//!   region::protect(ret5.as_ptr(), ret5.len(), Protection::ReadWriteExecute)?;
//!
//!   // VirtualLock | mlock
//!   let guard = region::lock(ret5.as_ptr(), ret5.len())?;
//!   # Ok(())
//!   # }
//!   ```
//!
//! - Using `View` to retrieve and change the state of memory pages.
//!
//!   ```rust
//!   # use region::{Access, View, Protection};
//!   let data = vec![0xFF; 100];
//!   let mut view = View::new(data.as_ptr(), data.len()).unwrap();
//!
//!   // Change memory protection to Read | Write | Execute
//!   unsafe { view.set_prot(Protection::ReadWriteExecute).unwrap() };
//!   assert_eq!(view.get_prot(), Some(Protection::ReadWriteExecute));
//!
//!   // Restore to the previous memory protection
//!   unsafe { view.set_prot(Access::Previous).unwrap() };
//!   assert_eq!(view.get_prot(), Some(Protection::ReadWrite));
//!
//!   // Temporarily change memory protection
//!   unsafe {
//!       view.exec_with_prot(Protection::Read, || {
//!           // This would result in a memory violation
//!           // data[0] = 0xCC;
//!       }).unwrap();
//!   }
//!
//!   // Lock the memory page(s) to RAM
//!   let _guard = view.lock().unwrap();
//!   ```

#[macro_use]
extern crate bitflags;
extern crate libc;

pub use crate::error::{Error, Result};
pub use crate::lock::{lock, unlock, LockGuard};
pub use crate::protection::Protection;
pub use crate::view::{Access, View};

mod error;
mod lock;
mod os;
pub mod page;
mod protection;
mod view;

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

/// Queries the OS with an address, returning the region it resides within.
///
/// The implementation uses `VirtualQuery` on Windows, `mach_vm_region` on macOS
/// and by parsing `proc/[pid]/maps` on Linux.
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
/// assert_eq!(region.protection, Protection::ReadWrite);
/// ```
pub fn query(address: *const u8) -> Result<Region> {
  if address.is_null() {
    Err(Error::Null)?;
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

/// Changes the memory protection of one or more pages.
///
/// The address range may overlap one or more pages, and if so, all pages
/// within the range will be modified. The previous protection flags are not
/// preserved (to reset protection flags to their inital values, `query_range`
/// can be used prior to this call, or by using a `View`).
///
/// If the size is zero this will affect the whole page located at the address
///
/// - The range is `[address, address + size)`
/// - The address may not be null.
/// - The address is rounded down to the closest page boundary.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Examples
///
/// ```
/// use region::{Protection};
///
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
/// let x: extern "C" fn() -> i32 = unsafe {
/// region::protect(ret5.as_ptr(), ret5.len(),
/// Protection::ReadWriteExecute).unwrap();   std::mem::transmute(ret5.as_ptr())
/// };
/// assert_eq!(x(), 5);
/// ```
pub unsafe fn protect(address: *const u8, size: usize, protection: Protection) -> Result<()> {
  if address.is_null() {
    Err(Error::Null)?;
  }

  // Ignore the preservation of previous protection flags
  os::set_protection(
    page::floor(address as usize) as *const u8,
    page::size_from_range(address, size),
    protection,
  )
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
  #[cfg(unix)]
  fn query_code() {
    // TODO: Find out why this fails on Windows
    let region = query(&query_code as *const _ as *const u8).unwrap();

    assert_eq!(region.guarded, false);
    assert_eq!(region.protection, Protection::ReadExecute);
    assert_eq!(region.shared, false);
  }

  #[test]
  fn query_alloc() {
    let size = page::size() * 2;
    let mut map = alloc_pages(&[Protection::ReadExecute, Protection::ReadExecute]);
    let region = query(map.as_ptr()).unwrap();

    assert_eq!(region.guarded, false);
    assert_eq!(region.protection, Protection::ReadExecute);
    assert!(!region.base.is_null() && region.base <= map.as_mut_ptr());
    assert!(region.size >= size);
  }

  #[test]
  fn query_area_zero() {
    let region = query_range(&query_area_zero as *const _ as *const u8, 0).unwrap();
    assert_eq!(region.len(), 1);
  }

  #[test]
  fn query_area_overlap() {
    let pz = page::size();
    let prots = [Protection::ReadExecute, Protection::ReadWrite];
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
      Protection::Read,
      Protection::ReadWrite,
      Protection::ReadExecute,
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

  #[test]
  fn protect_null() {
    assert!(unsafe { protect(::std::ptr::null(), 0, Protection::None) }.is_err());
  }

  #[test]
  fn protect_code() {
    let address = &mut protect_code as *mut _ as *mut u8;
    unsafe {
      protect(address, 0x10, Protection::ReadWriteExecute).unwrap();
      *address = 0x90;
    }
  }

  #[test]
  fn protect_alloc() {
    let mut map = alloc_pages(&[Protection::Read]);
    unsafe {
      protect(map.as_ptr(), page::size(), Protection::ReadWrite).unwrap();
      *map.as_mut_ptr() = 0x1;
    }
  }

  #[test]
  fn protect_overlap() {
    let pz = page::size();

    // Create a page boundary with different protection flags in the
    // upper and lower span, so the intermediate page sizes are fixed.
    let prots = [
      Protection::Read,
      Protection::ReadExecute,
      Protection::ReadWrite,
      Protection::Read,
    ];

    let map = alloc_pages(&prots);
    let base_exec = unsafe { map.as_ptr().offset(pz as isize) };
    let straddle = unsafe { base_exec.offset(pz as isize - 1) };

    // Change the protection over two page boundaries
    unsafe { protect(straddle, 2, Protection::ReadWriteExecute).unwrap() };

    // Ensure that the pages have merged into one region
    let result = query_range(base_exec, pz * 2).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].protection, Protection::ReadWriteExecute);
    assert_eq!(result[0].size, pz * 2);
  }
}
