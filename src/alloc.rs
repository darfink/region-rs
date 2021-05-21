use crate::{os, page, util, Error, Protection, Result};

/// A handle to a mapped memory region.
///
/// This handle does not dereference to a slice, since the underlying memory may
/// have been created with [Protection::NONE].
#[allow(clippy::len_without_is_empty)]
pub struct Memory {
  base: *const (),
  size: usize,
}

impl Memory {
  /// Returns a pointer to the allocation's base address.
  ///
  /// The address is always aligned to the operating system's page size.
  pub fn as_ptr<T>(&self) -> *const T {
    self.base as *const T
  }

  /// Returns a range spanning the allocation's address space.
  pub fn as_range(&self) -> std::ops::Range<usize> {
    (self.base as usize)..(self.base as usize).saturating_add(self.size)
  }

  /// Returns two raw pointers spanning the allocation's address space.
  ///
  /// The returned range is half-open, which means that the end pointer points
  /// one past the last element of the region. This way, an empty region is
  /// represented by two equal pointers, and the difference between the two
  /// pointers represents the size of the region.
  pub fn as_ptr_range<T>(&self) -> std::ops::Range<*const T> {
    let range = self.as_range();
    (range.start as *const T)..(range.end as *const T)
  }

  /// Returns the size of the allocation.
  ///
  /// The size is always aligned to the operating system's page size.
  pub fn len(&self) -> usize {
    self.size
  }
}

impl Drop for Memory {
  fn drop(&mut self) {
    let result = unsafe { os::free(self.base, self.size) };
    debug_assert!(result.is_ok(), "freeing region: {:?}", result);
  }
}

/// Allocates one or more pages of memory with defined a protection.
pub fn alloc(size: usize, protection: Protection) -> Result<Memory> {
  if size == 0 {
    return Err(Error::InvalidParameter("size"));
  }

  let size = page::ceil(size as *const ()) as usize;

  unsafe {
    let base = os::alloc(std::ptr::null::<()>(), size, protection)?;
    Ok(Memory { base, size })
  }
}

/// Allocates one or more pages of memory, at a specific address, with a defined
/// protection.
///
/// The returned memory allocation is not guaranteed to reside at the provided
/// address (for all operating systems). E.g. on Windows, new allocations that
/// do not reside within already reserved memory, are aligned to the operating
/// system's allocation granularity (in most cases 64KB).
pub fn alloc_at<T>(address: *const T, size: usize, protection: Protection) -> Result<Memory> {
  let (address, size) = util::round_to_page_boundaries(address, size)?;

  unsafe {
    let base = os::alloc(address as *const (), size, protection)?;
    Ok(Memory { base, size })
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
  fn alloc_rejects_empty_allocation() -> Result<()> {
    assert!(matches!(
      alloc(0, Protection::NONE),
      Err(Error::InvalidParameter(_))
    ));
    Ok(())
  }

  #[test]
  fn alloc_obtains_correct_properties() -> Result<()> {
    let memory = alloc(1, Protection::READ_WRITE)?;

    let region = crate::query(memory.as_ptr::<()>())?;
    assert_eq!(region.protection(), Protection::READ_WRITE);
    assert_eq!(region.len(), memory.len());
    assert!(!region.is_guarded());
    assert!(!region.is_shared());
    assert!(region.is_committed());

    Ok(())
  }

  #[test]
  fn alloc_frees_memory_when_dropped() -> Result<()> {
    let base = alloc(1, Protection::READ_WRITE)?.as_ptr::<()>();
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
}
