use std::mem::ManuallyDrop;

use crate::{os, page, util, Error, Protection, Result};

/// A handle to an owned region of memory.
///
/// This handle does not dereference to a slice, since the underlying memory may
/// have been created with [`Protection::NONE`].
#[allow(clippy::len_without_is_empty)]
pub struct Allocation {
  base: *const (),
  size: usize,
}

impl Allocation {
  /// Returns a pointer to the allocation's base address.
  ///
  /// The address is always aligned to the operating system's page size.
  #[inline(always)]
  pub fn as_ptr<T>(&self) -> *const T {
    self.base.cast()
  }

  /// Returns a mutable pointer to the allocation's base address.
  #[inline(always)]
  pub fn as_mut_ptr<T>(&mut self) -> *mut T {
    self.base as *mut T
  }

  /// Returns two raw pointers spanning the allocation's address space.
  ///
  /// The returned range is half-open, which means that the end pointer points
  /// one past the last element of the allocation. This way, an empty allocation
  /// is represented by two equal pointers, and the difference between the two
  /// pointers represents the size of the allocation.
  #[inline(always)]
  pub fn as_ptr_range<T>(&self) -> std::ops::Range<*const T> {
    let range = self.as_range();
    (range.start as *const T)..(range.end as *const T)
  }

  /// Returns two mutable raw pointers spanning the allocation's address space.
  #[inline(always)]
  pub fn as_mut_ptr_range<T>(&mut self) -> std::ops::Range<*mut T> {
    let range = self.as_range();
    (range.start as *mut T)..(range.end as *mut T)
  }

  /// Returns a range spanning the allocation's address space.
  #[inline(always)]
  pub fn as_range(&self) -> std::ops::Range<usize> {
    (self.base as usize)..(self.base as usize).saturating_add(self.size)
  }

  /// Returns the size of the allocation in bytes.
  ///
  /// The size is always aligned to a multiple of the operating system's page
  /// size.
  #[inline(always)]
  pub fn len(&self) -> usize {
    self.size
  }

  /// Decomposes an `Allocation` into its raw components: `(pointer, length)`.
  ///
  /// After calling this function, the caller is responsible for the previously
  /// managed allocation.
  ///
  /// For creating an `Allocation` from raw components, see [`Self::from_raw_parts`].
  #[inline]
  pub fn into_raw_parts<T>(self) -> (*mut T, usize) {
    let mut this = ManuallyDrop::new(self);
    (this.as_mut_ptr(), this.len())
  }

  /// Creates a `Allocation` directly from a pointer, and a length.
  ///
  /// For decomposing an `Allocation` into raw components, see
  /// [`Self::into_raw_parts`].
  ///
  /// # Safety
  ///
  /// This is highly unsafe because given `ptr` and `length` could not
  /// be checked as valid allocation, and the caller should guarantee
  /// that they are valid parts.
  #[inline(always)]
  pub unsafe fn from_raw_parts<T>(ptr: *mut T, length: usize) -> Self {
    Self {
      base: ptr as *const (),
      size: length,
    }
  }
}

impl Drop for Allocation {
  #[inline]
  fn drop(&mut self) {
    let result = unsafe { os::free(self.base, self.size) };
    debug_assert!(result.is_ok(), "freeing region: {:?}", result);
  }
}

/// Allocates one or more pages of memory, with a defined protection.
///
/// This function provides a very simple interface for allocating anonymous
/// virtual pages. The allocation address will be decided by the operating
/// system.
///
/// # Parameters
///
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary.
///
/// # Errors
///
/// - If an interaction with the underlying operating system fails, an error
/// will be returned.
/// - If size is zero, [`Error::InvalidParameter`] will be returned.
///
/// # OS-Specific Behavior
///
/// On NetBSD pages will be allocated without PaX memory protection restrictions
/// (i.e. pages will be allowed to be modified to any combination of `RWX`).
///
/// # Examples
///
/// ```
/// # fn main() -> region::Result<()> {
/// # if cfg!(any(target_arch = "x86", target_arch = "x86_64"))
/// #   && !cfg!(any(target_os = "openbsd", target_os = "netbsd")) {
/// use region::Protection;
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3u8];
///
/// let memory = region::alloc(100, Protection::READ_WRITE_EXECUTE)?;
/// let slice = unsafe {
///   std::slice::from_raw_parts_mut(memory.as_ptr::<u8>() as *mut u8, memory.len())
/// };
///
/// slice[..6].copy_from_slice(&ret5);
/// let x: extern "C" fn() -> i32 = unsafe { std::mem::transmute(slice.as_ptr()) };
///
/// assert_eq!(x(), 5);
/// # }
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn alloc(size: usize, protection: Protection) -> Result<Allocation> {
  if size == 0 {
    return Err(Error::InvalidParameter("size"));
  }

  let size = page::ceil(size as *const ()) as usize;

  unsafe {
    let base = os::alloc(std::ptr::null::<()>(), size, protection)?;
    Ok(Allocation { base, size })
  }
}

/// Allocates one or more pages of memory, at a specific address, with a defined
/// protection.
///
/// The returned memory allocation is not guaranteed to reside at the provided
/// address. E.g. on Windows, new allocations that do not reside within already
/// reserved memory, are aligned to the operating system's allocation
/// granularity (most commonly 64KB).
///
/// # Implementation
///
/// This function is implemented using `VirtualAlloc` on Windows, and `mmap`
/// with `MAP_FIXED` on POSIX.
///
/// # Parameters
///
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Errors
///
/// - If an interaction with the underlying operating system fails, an error
/// will be returned.
/// - If size is zero, [`Error::InvalidParameter`] will be returned.
#[inline]
pub fn alloc_at<T>(address: *const T, size: usize, protection: Protection) -> Result<Allocation> {
  let (address, size) = util::round_to_page_boundaries(address, size)?;

  unsafe {
    let base = os::alloc(address.cast(), size, protection)?;
    Ok(Allocation { base, size })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alloc_size_is_aligned_to_page_size() -> Result<()> {
    let memory = alloc(1, Protection::NONE)?;
    assert_eq!(memory.len(), page::size());
    Ok(())
  }

  #[test]
  fn alloc_rejects_empty_allocation() {
    assert!(matches!(
      alloc(0, Protection::NONE),
      Err(Error::InvalidParameter(_))
    ));
  }

  #[test]
  fn alloc_obtains_correct_properties() -> Result<()> {
    let memory = alloc(1, Protection::READ_WRITE)?;

    let region = crate::query(memory.as_ptr::<()>())?;
    assert_eq!(region.protection(), Protection::READ_WRITE);
    assert!(region.len() >= memory.len());
    assert!(!region.is_guarded());
    assert!(!region.is_shared());
    assert!(region.is_committed());

    Ok(())
  }

  #[test]
  fn alloc_frees_memory_when_dropped() -> Result<()> {
    // Designing these tests can be quite tricky sometimes. When a page is
    // allocated and then released, a subsequent `query` may allocate memory in
    // the same location that has just been freed. For instance, NetBSD's
    // kinfo_getvmmap uses `mmap` internally, which can lead to potentially
    // confusing outcomes. To mitigate this, an additional buffer region is
    // allocated to ensure that any memory allocated indirectly through `query`
    // occupies a separate location in memory.
    let (start, _buffer) = (
      alloc(1, Protection::READ_WRITE)?,
      alloc(1, Protection::READ_WRITE)?,
    );

    let base = start.as_ptr::<()>();
    std::mem::drop(start);

    let query = crate::query(base);
    assert!(matches!(query, Err(Error::UnmappedRegion)));
    Ok(())
  }

  #[test]
  fn alloc_can_allocate_unused_region() -> Result<()> {
    let base = alloc(1, Protection::NONE)?.as_ptr::<()>();
    let memory = alloc_at(base, 1, Protection::READ_WRITE)?;
    assert_eq!(memory.as_ptr(), base);
    Ok(())
  }

  #[test]
  #[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
  fn alloc_can_allocate_executable_region() -> Result<()> {
    let memory = alloc(1, Protection::WRITE_EXECUTE)?;
    assert_eq!(memory.len(), page::size());
    Ok(())
  }
}
