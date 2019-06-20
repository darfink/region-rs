use {os, page, query_range, Error, Region, Result};

/// Changes the memory protection of one or more pages.
///
/// The address range may overlap one or more pages, and if so, all pages within
/// the range will be modified. The previous protection flags are not preserved
/// (if reset of protection flags is desired, use `protect_with_handle`).
///
/// - The range is `[address, address + size)`
/// - The address may not be null.
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Examples
///
/// ```
/// # if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
/// use region::{Protection};
///
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
/// let x: extern "C" fn() -> i32 = unsafe {
///   region::protect(ret5.as_ptr(), ret5.len(), Protection::ReadWriteExecute).unwrap();
///   std::mem::transmute(ret5.as_ptr())
/// };
/// assert_eq!(x(), 5);
/// # }
/// ```
pub unsafe fn protect(address: *const u8, size: usize, protection: Protection) -> Result<()> {
  if address.is_null() {
    Err(Error::NullAddress)?;
  }

  if size == 0 {
    Err(Error::EmptyRange)?;
  }

  // Ignore the preservation of previous protection flags
  os::set_protection(
    page::floor(address as usize) as *const u8,
    page::size_from_range(address, size),
    protection,
  )
}

/// Changes the memory protection of one or more pages temporarily.
///
/// The address range may overlap one or more pages, and if so, all pages within
/// the range will be modified. The protection flags will be reset when the
/// handle is dropped.
///
/// This function uses `query_range` internally and is therefore less performant
/// than `protect`. Prefer this function only if a memory protection reset is
/// desired.
///
/// - The range is `[address, address + size)`
/// - The address may not be null.
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
pub unsafe fn protect_with_handle(
  address: *const u8,
  size: usize,
  protection: Protection,
) -> Result<ProtectGuard> {
  // Determine the current region flags
  let mut regions = query_range(address, size)?;

  // Change the region to the desired protection
  protect(address, size, protection)?;

  let lower = page::floor(address as usize);
  let upper = page::ceil(address as usize + size);

  if let Some(ref mut region) = regions.first_mut() {
    // Offset the lower region to the smallest page boundary
    let delta = lower - region.base as usize;
    region.base = (region.base as usize + delta) as *mut u8;
    region.size -= delta;
  }

  if let Some(ref mut region) = regions.last_mut() {
    // Truncate the upper region to the smallest page boundary
    let delta = region.upper() - upper;
    region.size -= delta;
  }

  Ok(ProtectGuard::new(regions))
}

/// An RAII implementation of "scoped protection". When this structure is dropped
/// (falls out of scope), the memory region protection will be reset.
#[must_use]
pub struct ProtectGuard {
  regions: Vec<Region>,
}

impl ProtectGuard {
  fn new(regions: Vec<Region>) -> Self {
    ProtectGuard { regions }
  }

  /// Releases the guards ownership of the memory protection.
  pub unsafe fn release(self) {
    ::std::mem::forget(self);
  }
}

impl Drop for ProtectGuard {
  fn drop(&mut self) {
    let result = unsafe {
      self
        .regions
        .iter()
        .try_for_each(|region| protect(region.base, region.size, region.protection))
    };
    debug_assert!(result.is_ok(), "restoring region protection");
  }
}

unsafe impl Send for ProtectGuard {}
unsafe impl Sync for ProtectGuard {}

bitflags! {
  /// Memory page protection constants.
  ///
  /// Determines the access rights for a specific page and/or region. Some
  /// combination of flags may not work depending on the OS (e.g macOS
  /// enforces pages to be readable).
  ///
  /// # Examples
  ///
  /// ```
  /// use region::Protection;
  ///
  /// let combine = Protection::Read | Protection::Write;
  /// let shorthand = Protection::ReadWrite;
  /// ```
  pub struct Protection: usize {
    /// No access allowed at all.
    const None             = 0;
    /// Read access; writing and/or executing data will panic.
    const Read             = (1 << 1);
    /// Write access; this flag alone may not be supported on all OSs.
    const Write            = (1 << 2);
    /// Execute access; this may not be allowed depending on DEP.
    const Execute          = (1 << 3);
    /// Read and execute shorthand.
    const ReadExecute      = (Self::Read.bits | Self::Execute.bits);
    /// Read and write shorthand.
    const ReadWrite        = (Self::Read.bits | Self::Write.bits);
    /// Read, write and execute shorthand.
    const ReadWriteExecute = (Self::Read.bits | Self::Write.bits | Self::Execute.bits);
    /// Write and execute shorthand.
    const WriteExecute     = (Self::Write.bits | Self::Execute.bits);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tests::alloc_pages;

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

  #[test]
  fn protect_handle() {
    let map = alloc_pages(&[Protection::Read]);
    unsafe {
      let _handle = protect_with_handle(map.as_ptr(), page::size(), Protection::ReadWrite).unwrap();
      assert_eq!(
        ::query(map.as_ptr()).unwrap().protection,
        Protection::ReadWrite
      );
    };
    assert_eq!(::query(map.as_ptr()).unwrap().protection, Protection::Read);
  }
}
