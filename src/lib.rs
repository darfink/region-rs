//! Cross-platform virtual memory API.
//!
//! This crate provides a cross-platform Rust API for querying and manipulating
//! virtual memory. It is a thin abstraction, with the underlying interaction
//! implemented using platform specific APIs (e.g `VirtualQuery`, `VirtualLock`,
//! `mprotect`, `mlock`). Albeit not all OS specific quirks are abstracted away;
//! for instance, some OSs enforce memory pages to be readable, whilst other may
//! prevent pages from becoming executable (i.e DEP).
//!
//! This implementation operates with memory pages, which are aligned to the
//! operating system's page size. On some systems, but not all, the system calls
//! for these operations require input to be aligned to a page boundary. To
//! remedy this inconsistency, whenever applicable, input is aligned to its
//! closest page boundary.
//!
//! *Note: a region is a collection of one or more pages laying consecutively in
//! memory, with the same properties.*
//!
//! # Parallelism
//!
//! The properties of virtual memory pages can change at any time, unless all
//! threads that are unaccounted for in a process are stopped. Therefore to
//! obtain, e.g., a true picture of a process' virtual memory, all other threads
//! must be halted. Otherwise, a region descriptor only represents a snapshot in
//! time.
//!
//! # Installation
//!
//! This crate is [on crates.io](https://crates.io/crates/region) and can be
//! used by adding `region` to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! region = "4.0.1"
//! ```
//!
//! # Features
//!
//! By default this crate enables the `std` feature. Disable default features to
//! use it as `#![no_std]` with the `alloc` crate:
//!
//! ```toml
//! [dependencies]
//! region = { version = "4.0.1", default-features = false }
//! ```
//!
//! With `std` enabled, [`Error`] implements [`std::error::Error`].
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
//!   // VirtualAlloc | mmap
//!   let alloc = region::alloc(100, Protection::READ_WRITE)?;
//!
//!   // VirtualProtect | mprotect
//!   region::protect(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
//!
//!   // ... you can also temporarily change one or more pages' protection
//!   let handle = region::protect_with_handle(data.as_ptr(), data.len(), Protection::READ_WRITE_EXECUTE)?;
//!
//!   // VirtualLock | mlock
//!   let guard = region::lock(data.as_ptr(), data.len())?;
//!   # Ok(())
//!   # }
//!   ```

#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub use allocation::{Allocation, alloc, alloc_at};
pub use error::{Error, Result};
pub use lock::{LockGuard, lock, unlock};
pub use protect::{ProtectGuard, protect, protect_with_handle};
pub use query::{QueryIter, query, query_range};

mod allocation;
mod error;
mod lock;
mod os;
pub mod page;
mod protect;
mod query;
mod util;

/// A descriptor for a mapped memory region.
///
/// The region encompasses zero or more pages (e.g. OpenBSD can have null-sized
/// virtual pages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
  /// Base address of the region
  base: *const (),
  /// Whether the region is reserved or not
  reserved: bool,
  /// Whether the region is guarded or not
  guarded: bool,
  /// Protection of the region
  protection: Protection,
  /// Maximum protection of the region
  max_protection: Protection,
  /// Whether the region is shared or not
  shared: bool,
  /// Size of the region (multiple of page size)
  size: usize,
}

impl Region {
  /// Returns a pointer to the region's base address.
  ///
  /// The address is always aligned to the operating system's page size.
  #[inline(always)]
  pub fn as_ptr<T>(&self) -> *const T {
    self.base.cast()
  }

  /// Returns a mutable pointer to the region's base address.
  #[inline(always)]
  pub fn as_mut_ptr<T>(&mut self) -> *mut T {
    self.base.cast_mut().cast()
  }

  /// Returns two raw pointers spanning the region's address space.
  ///
  /// The returned range is half-open, which means that the end pointer points
  /// one past the last element of the region. This way, an empty region is
  /// represented by two equal pointers, and the difference between the two
  /// pointers represents the size of the region.
  #[inline(always)]
  pub fn as_ptr_range<T>(&self) -> core::ops::Range<*const T> {
    let range = self.as_range();
    core::ptr::with_exposed_provenance::<T>(range.start)
      ..core::ptr::with_exposed_provenance::<T>(range.end)
  }

  /// Returns two mutable raw pointers spanning the region's address space.
  #[inline(always)]
  pub fn as_mut_ptr_range<T>(&mut self) -> core::ops::Range<*mut T> {
    let range = self.as_range();
    core::ptr::with_exposed_provenance_mut::<T>(range.start)
      ..core::ptr::with_exposed_provenance_mut::<T>(range.end)
  }

  /// Returns a range spanning the region's address space.
  #[inline(always)]
  pub fn as_range(&self) -> core::ops::Range<usize> {
    let start = self.base.addr();
    start..start.saturating_add(self.size)
  }

  /// Returns whether the region is committed or not.
  ///
  /// This is always true for all operating system's, the exception being
  /// `MEM_RESERVE` pages on Windows.
  #[inline(always)]
  pub fn is_committed(&self) -> bool {
    !self.reserved
  }

  /// Returns whether the region is readable or not.
  #[inline(always)]
  pub fn is_readable(&self) -> bool {
    self.protection.contains(Protection::READ)
  }

  /// Returns whether the region is writable or not.
  #[inline(always)]
  pub fn is_writable(&self) -> bool {
    self.protection.contains(Protection::WRITE)
  }

  /// Returns whether the region is executable or not.
  #[inline(always)]
  pub fn is_executable(&self) -> bool {
    self.protection.contains(Protection::EXECUTE)
  }

  /// Returns whether the region is guarded or not.
  #[inline(always)]
  pub fn is_guarded(&self) -> bool {
    self.guarded
  }

  /// Returns whether the region is shared between processes or not.
  #[inline(always)]
  pub fn is_shared(&self) -> bool {
    self.shared
  }

  /// Returns the size of the region in bytes.
  ///
  /// The size is always aligned to a multiple of the operating system's page
  /// size.
  #[inline(always)]
  pub fn len(&self) -> usize {
    self.size
  }

  /// Returns whether region is empty or not.
  #[inline(always)]
  pub fn is_empty(&self) -> bool {
    self.size == 0
  }

  /// Returns the protection attributes of the region.
  #[inline(always)]
  pub fn protection(&self) -> Protection {
    self.protection
  }

  /// Returns the maximum protection attributes of the region.
  ///
  /// On platforms that do not expose this information, this matches
  /// [`Self::protection`].
  #[inline(always)]
  pub fn max_protection(&self) -> Protection {
    self.max_protection
  }
}

impl Default for Region {
  #[inline]
  fn default() -> Self {
    Self {
      base: core::ptr::null(),
      reserved: false,
      guarded: false,
      protection: Protection::NONE,
      max_protection: Protection::NONE,
      shared: false,
      size: 0,
    }
  }
}

unsafe impl Send for Region {}
unsafe impl Sync for Region {}

bitflags::bitflags! {
  /// A bitflag of zero or more protection attributes.
  ///
  /// Determines the access rights for a specific page and/or region. Some
  /// combination of flags may not be applicable, depending on the OS (e.g macOS
  /// enforces executable pages to be readable, OpenBSD requires W^X).
  ///
  /// # OS-Specific Behavior
  ///
  /// On Unix [`Protection::from_bits_retain`] can be used to apply non-standard
  /// flags (e.g. `PROT_BTI`).
  ///
  /// # Examples
  ///
  /// ```
  /// use region::Protection;
  ///
  /// let combine = Protection::READ | Protection::WRITE;
  /// let shorthand = Protection::READ_WRITE;
  /// ```
  #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
  pub struct Protection: usize {
    /// No access allowed at all.
    const NONE = 0;
    /// Read access; writing and/or executing data will panic.
    const READ = 1 << 0;
    /// Write access; this flag alone may not be supported on all OSs.
    const WRITE = 1 << 1;
    /// Execute access; this may not be allowed depending on DEP.
    const EXECUTE = 1 << 2;
    /// Read and execute shorthand.
    const READ_EXECUTE = Self::READ.bits() | Self::EXECUTE.bits();
    /// Read and write shorthand.
    const READ_WRITE = Self::READ.bits() | Self::WRITE.bits();
    /// Read, write and execute shorthand.
    const READ_WRITE_EXECUTE = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    /// Write and execute shorthand.
    const WRITE_EXECUTE = Self::WRITE.bits() | Self::EXECUTE.bits();
  }
}

impl Protection {
  /// Convert from underlying bit representation, preserving all bits (even
  /// those not corresponding to a defined flag).
  /// Convert from underlying bit representation, preserving all bits (even
  /// those not corresponding to a defined flag).
  ///
  /// # Safety
  ///
  /// This method is identical to the safe [`Self::from_bits_retain`] and exists
  /// only for compatibility with older callers. Prefer the safe method.
  #[deprecated = "use the safe `from_bits_retain` method instead"]
  #[inline(always)]
  pub const unsafe fn from_bits_unchecked(bits: usize) -> Self {
    Self::from_bits_retain(bits)
  }
}

impl core::fmt::Display for Protection {
  #[inline]
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    const MAPPINGS: &[(Protection, char)] = &[
      (Protection::READ, 'r'),
      (Protection::WRITE, 'w'),
      (Protection::EXECUTE, 'x'),
    ];

    for (flag, symbol) in MAPPINGS {
      if self.contains(*flag) {
        write!(f, "{symbol}")?;
      } else {
        write!(f, "-")?;
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use alloc::string::ToString;

  #[test]
  fn protection_implements_display() {
    assert_eq!(Protection::READ.to_string(), "r--");
    assert_eq!(Protection::READ_WRITE.to_string(), "rw-");
    assert_eq!(Protection::READ_WRITE_EXECUTE.to_string(), "rwx");
    assert_eq!(Protection::WRITE.to_string(), "-w-");
  }

  #[cfg(unix)]
  pub mod util {
    use crate::{Protection, page};
    use alloc::vec;
    use alloc::vec::Vec;
    use core::ops::Deref;
    use mmap::{MapOption, MemoryMap};

    struct AllocatedPages(Vec<MemoryMap>);

    impl Deref for AllocatedPages {
      type Target = [u8];

      #[inline]
      fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.0[0].data().cast(), self.0.len() * page::size()) }
      }
    }

    #[allow(clippy::fallible_impl_from)]
    impl From<Protection> for &'static [MapOption] {
      #[inline]
      fn from(protection: Protection) -> Self {
        match protection {
          Protection::NONE => &[],
          Protection::READ => &[MapOption::MapReadable],
          Protection::READ_WRITE => &[MapOption::MapReadable, MapOption::MapWritable],
          Protection::READ_EXECUTE => &[MapOption::MapReadable, MapOption::MapExecutable],
          _ => panic!("Unsupported protection {:?}", protection),
        }
      }
    }

    /// Allocates one or more sequential pages for each protection flag.
    pub fn alloc_pages(pages: &[Protection]) -> impl Deref<Target = [u8]> {
      // Find a region that fits all pages
      let region = MemoryMap::new(page::size() * pages.len(), &[]).expect("allocating pages");
      let mut page_address = region.data();

      // Drop the region to ensure it's free
      core::mem::forget(region);

      // Allocate one page at a time, with explicit page permissions. This would
      // normally introduce a race condition, but since only one thread is used
      // during testing, it ensures each page remains available (in general,
      // only one thread should ever be active when querying and/or manipulating
      // memory regions).
      let allocated_pages = pages
        .iter()
        .map(|protection| {
          let mut options = vec![MapOption::MapAddr(page_address)];
          options.extend_from_slice(Into::into(*protection));

          let map = MemoryMap::new(page::size(), &options).expect("allocating page");
          assert_eq!(map.data(), page_address);
          assert_eq!(map.len(), page::size());

          page_address = unsafe { page_address.add(page::size()) };
          map
        })
        .collect::<Vec<_>>();

      AllocatedPages(allocated_pages)
    }
  }

  #[cfg(windows)]
  pub mod util {
    use crate::{Protection, page};
    use core::ops::Deref;
    use windows_sys::Win32::System::Memory::{
      MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, VirtualAlloc, VirtualFree,
    };

    struct AllocatedPages(*const (), usize);

    impl Deref for AllocatedPages {
      type Target = [u8];

      #[inline]
      fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.0.cast(), self.1) }
      }
    }

    impl Drop for AllocatedPages {
      #[inline]
      fn drop(&mut self) {
        unsafe {
          assert_ne!(VirtualFree(self.0 as *mut _, 0, MEM_RELEASE), 0);
        }
      }
    }

    /// Allocates one or more sequential pages for each protection flag.
    pub fn alloc_pages(pages: &[Protection]) -> impl Deref<Target = [u8]> {
      // Reserve enough memory to fit each page
      let total_size = page::size() * pages.len();
      let allocation_base = unsafe {
        VirtualAlloc(
          core::ptr::null_mut(),
          total_size,
          MEM_RESERVE,
          PAGE_NOACCESS,
        )
      };
      assert_ne!(allocation_base, core::ptr::null_mut());

      let mut page_address = allocation_base;

      // Commit one page at a time with the expected permissions
      for protection in pages {
        let address = unsafe {
          VirtualAlloc(
            page_address,
            page::size(),
            MEM_COMMIT,
            protection.to_native(),
          )
        };
        assert_eq!(address, page_address);
        page_address = unsafe { address.cast::<u8>().add(page::size()) }.cast();
      }

      AllocatedPages(allocation_base.cast(), total_size)
    }
  }
}
