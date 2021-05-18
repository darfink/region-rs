use crate::{os, round_to_page_boundaries, QueryIter, Region, Result};

/// Changes the memory protection of one or more pages.
///
/// The address range may overlap one or more pages, and if so, all pages within
/// the range will be modified. The previous protection flags are not preserved
/// (if you desire to preserve the protection flags, use [protect_with_handle]).
///
/// # Parameters
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Safety
///
/// This function can violate memory safety in a myriad of ways. Read-only memory
/// can become writable, the executable properties of code segments can be
/// removed, etc.
///
/// # Examples
///
/// - Make an array of x86 assembly instructions executable.
///
/// ```
/// # fn main() -> region::Result<()> {
/// # if cfg!(any(target_arch = "x86", target_arch = "x86_64")) && !cfg!(target_os = "openbsd") {
/// use region::Protection;
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3u8];
///
/// let x: extern "C" fn() -> i32 = unsafe {
///   region::protect(ret5.as_ptr(), ret5.len(), region::Protection::READ_WRITE_EXECUTE)?;
///   std::mem::transmute(ret5.as_ptr())
/// };
///
/// assert_eq!(x(), 5);
/// # }
/// # Ok(())
/// # }
/// ```
pub unsafe fn protect<T>(address: *const T, size: usize, protection: Protection) -> Result<()> {
  let (address, size) = round_to_page_boundaries(address, size)?;
  os::protect(address, size, protection)
}

/// Temporarily changes the memory protection of one or more pages.
///
/// The address range may overlap one or more pages, and if so, all pages within
/// the range will be modified. The protection flag for each page will be reset
/// once the handle is dropped. To conditionally prevent a reset, use
/// [std::mem::forget].
///
/// This function uses [query_range](crate::query_range) internally and is
/// therefore less performant than [protect]. Use this function only if you need
/// to reapply the memory protection flags of one or more regions after
/// operations.
///
/// # Guard
///
/// Remember not to conflate the *black hole* syntax with the ignored, but
/// unused, variable syntax. Otherwise the [ProtectGuard] instantly resets the
/// protection flags of all pages.
///
/// ```ignore
/// let _ = protect_with_handle(...);      // Pages are instantly reset
/// let _guard = protect_with_handle(...); // Pages are reset once `_guard` is dropped.
/// ```
///
/// # Parameters
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Safety
///
/// See [protect].
pub unsafe fn protect_with_handle<T>(
  address: *const T,
  size: usize,
  protection: Protection,
) -> Result<ProtectGuard> {
  let (address, size) = round_to_page_boundaries(address, size)?;

  // Preserve the current regions' flags
  let mut regions = QueryIter::new(address, size)?.collect::<Result<Vec<_>>>()?;

  // Apply the desired protection flags
  protect(address, size, protection)?;

  if let Some(region) = regions.first_mut() {
    // Offset the lower region to the smallest page boundary
    let delta = address as usize - region.as_ptr() as *const () as usize;
    region.base = (region.base as usize + delta) as *const _;
    region.size -= delta;
  }

  if let Some(ref mut region) = regions.last_mut() {
    // Truncate the upper region to the smallest page boundary
    let delta = (region.as_ptr() as *const () as usize + region.len()) - (address as usize + size);
    region.size -= delta;
  }

  Ok(ProtectGuard::new(regions))
}

/// An RAII implementation of a scoped protection guard.
///
/// When this structure is dropped (falls out of scope), the memory regions'
/// protection will be reset.
#[must_use]
pub struct ProtectGuard {
  regions: Vec<Region>,
}

impl ProtectGuard {
  fn new(regions: Vec<Region>) -> Self {
    ProtectGuard { regions }
  }
}

impl Drop for ProtectGuard {
  fn drop(&mut self) {
    let result = self
      .regions
      .iter()
      .try_for_each(|region| unsafe { protect(region.base, region.size, region.protection) });
    debug_assert!(result.is_ok(), "restoring region protection: {:?}", result);
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
  /// let combine = Protection::READ | Protection::WRITE;
  /// let shorthand = Protection::READ_WRITE;
  /// ```
  #[derive(Default)]
  pub struct Protection: usize {
    /// No access allowed at all.
    const NONE = 0;
    /// Read access; writing and/or executing data will panic.
    const READ = (1 << 1);
    /// Write access; this flag alone may not be supported on all OSs.
    const WRITE = (1 << 2);
    /// Execute access; this may not be allowed depending on DEP.
    const EXECUTE = (1 << 3);
    /// Read and execute shorthand.
    const READ_EXECUTE = (Self::READ.bits | Self::EXECUTE.bits);
    /// Read and write shorthand.
    const READ_WRITE = (Self::READ.bits | Self::WRITE.bits);
    /// Read, write and execute shorthand.
    const READ_WRITE_EXECUTE = (Self::READ.bits | Self::WRITE.bits | Self::EXECUTE.bits);
    /// Write and execute shorthand.
    const WRITE_EXECUTE = (Self::WRITE.bits | Self::EXECUTE.bits);
  }
}

impl std::fmt::Display for Protection {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    const MAPPINGS: &[(Protection, char)] = &[
      (Protection::READ, 'r'),
      (Protection::WRITE, 'w'),
      (Protection::EXECUTE, 'x'),
    ];

    for (flag, symbol) in MAPPINGS {
      if self.contains(*flag) {
        write!(f, "{}", symbol)?;
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
  use crate::tests::util::alloc_pages;
  use crate::{page, query, query_range};

  #[test]
  fn protect_null_fails() {
    assert!(unsafe { protect(std::ptr::null::<()>(), 0, Protection::NONE) }.is_err());
  }

  #[test]
  #[cfg(not(target_os = "openbsd"))]
  fn protect_can_alter_text_segments() {
    let address = &mut protect_can_alter_text_segments as *mut _ as *mut u8;
    unsafe {
      protect(address, 1, Protection::READ_WRITE_EXECUTE).unwrap();
      *address = 0x90;
    }
  }

  #[test]
  fn protect_updates_both_pages_for_straddling_range() -> Result<()> {
    let pz = page::size();

    // Create a page boundary with different protection flags in the upper and
    // lower span, so the intermediate region sizes are fixed to one page.
    let map = alloc_pages(&[
      Protection::READ,
      Protection::READ_EXECUTE,
      Protection::READ_WRITE,
      Protection::READ,
    ]);

    let base_exec = unsafe { map.as_ptr().add(pz) };
    let straddle = unsafe { base_exec.add(pz - 1) };

    // Change the protection over two page boundaries
    unsafe { protect(straddle, 2, Protection::NONE)? };

    // Query the two pages that reside within the outer two
    let result = query_range(base_exec, pz * 2)?.collect::<Result<Vec<_>>>()?;

    assert!(matches!(result.len(), 1 | 2));
    assert_eq!(result.iter().map(|r| r.len()).sum::<usize>(), pz * 2);
    assert_eq!(result[0].protection(), Protection::NONE);
    Ok(())
  }

  #[test]
  fn protect_has_inclusive_lower_and_exclusive_upper_bound() -> Result<()> {
    let map = alloc_pages(&[
      Protection::READ_WRITE,
      Protection::READ,
      Protection::READ_WRITE,
      Protection::READ,
    ]);

    // Alter the protection of the second page
    let second_page = unsafe { map.as_ptr().add(page::size()) };
    unsafe {
      let edge = second_page.offset(page::size() as isize - 1);
      protect(edge, 1, Protection::NONE)?;
    }

    let regions = query_range(map.as_ptr(), page::size() * 3)?.collect::<Result<Vec<_>>>()?;
    assert_eq!(regions.len(), 3);
    assert_eq!(regions[0].protection(), Protection::READ_WRITE);
    assert_eq!(regions[1].protection(), Protection::NONE);
    assert_eq!(regions[2].protection(), Protection::READ_WRITE);

    // Alter the protection of '2nd_page_start .. 2nd_page_end + 1'
    unsafe {
      protect(second_page, page::size() + 1, Protection::READ_EXECUTE)?;
    }

    let regions = query_range(map.as_ptr(), page::size() * 3)?.collect::<Result<Vec<_>>>()?;
    assert!(regions.len() >= 2);
    assert_eq!(regions[0].protection(), Protection::READ_WRITE);
    assert_eq!(regions[1].protection(), Protection::READ_EXECUTE);
    assert!(regions[1].len() >= page::size());

    Ok(())
  }

  #[test]
  fn protect_with_handle_resets_protection() -> Result<()> {
    let map = alloc_pages(&[Protection::READ]);

    unsafe {
      let _handle = protect_with_handle(map.as_ptr(), page::size(), Protection::READ_WRITE)?;
      assert_eq!(query(map.as_ptr())?.protection(), Protection::READ_WRITE);
    };

    assert_eq!(query(map.as_ptr())?.protection(), Protection::READ);
    Ok(())
  }

  #[test]
  fn protect_with_handle_only_resets_protection_of_affected_pages() -> Result<()> {
    let pages = [
      Protection::READ,
      Protection::READ,
      Protection::READ_WRITE,
      Protection::READ_EXECUTE,
      Protection::READ_EXECUTE,
    ];
    let map = alloc_pages(&pages);

    let second_page = unsafe { map.as_ptr().add(page::size()) };
    let region_size = page::size() * 3;

    unsafe {
      let _handle = protect_with_handle(second_page, region_size, Protection::NONE)?;
      let region = query(second_page)?;

      assert_eq!(region.protection(), Protection::NONE);
      assert_eq!(region.as_ptr(), second_page);
    }

    let regions =
      query_range(map.as_ptr(), page::size() * pages.len())?.collect::<Result<Vec<_>>>()?;
    assert!(matches!(regions.len(), 3 | 4 | 5));
    assert!(regions[0].as_ptr() <= map.as_ptr());
    assert_eq!(regions[0].protection(), Protection::READ);

    Ok(())
  }

  #[test]
  fn protection_implements_display() {
    assert_eq!(Protection::READ.to_string(), "r--");
    assert_eq!(Protection::READ_WRITE.to_string(), "rw-");
    assert_eq!(Protection::READ_WRITE_EXECUTE.to_string(), "rwx");
    assert_eq!(Protection::WRITE.to_string(), "-w-");
  }
}
