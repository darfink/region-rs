use {os, page, Error, Result};

/// Locks one or more memory regions to RAM.
///
/// The memory pages within the address range is guaranteed to stay in RAM
/// except for specials cases such as hibernation and memory starvation.
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
/// let data = [0; 100];
/// let _guard = region::lock(data.as_ptr(), data.len()).unwrap();
/// ```
pub fn lock(address: *const u8, size: usize) -> Result<LockGuard> {
  if address.is_null() {
    return Err(Error::NullAddress);
  }

  if size == 0 {
    return Err(Error::EmptyRange);
  }

  os::lock(
    page::floor(address as usize) as *const u8,
    page::size_from_range(address, size),
  )
  .map(|_| LockGuard::new(address, size))
}

/// Unlocks one or more memory regions from RAM.
///
/// - The range is `[address, address + size)`
/// - The address may not be null.
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Safety
///
/// This function is unsafe since it cannot be known whether it is called on a
/// locked region or not. In normal uses cases, the `LockGuard` is recommended
/// for safe code.
pub unsafe fn unlock(address: *const u8, size: usize) -> Result<()> {
  if address.is_null() {
    return Err(Error::NullAddress);
  }

  if size == 0 {
    return Err(Error::EmptyRange);
  }

  os::unlock(
    page::floor(address as usize) as *const u8,
    page::size_from_range(address, size),
  )
}

/// An RAII implementation of a "scoped lock". When this structure is dropped
/// (falls out of scope), the virtual lock will be unlocked.
#[must_use]
pub struct LockGuard {
  address: *const u8,
  size: usize,
}

impl LockGuard {
  fn new(address: *const u8, size: usize) -> Self {
    LockGuard { address, size }
  }

  /// Releases the guards ownership of the virtual lock.
  #[deprecated(since = "2.2.0", note = "Use std::mem::forget instead")]
  pub fn release(self) {
    ::std::mem::forget(self);
  }
}

impl Drop for LockGuard {
  fn drop(&mut self) {
    let result = unsafe { ::unlock(self.address, self.size) };
    debug_assert!(result.is_ok(), "unlocking region");
  }
}

unsafe impl Send for LockGuard {}
unsafe impl Sync for LockGuard {}

#[cfg(test)]
mod tests {
  use super::*;
  use os::page_size;
  use tests::alloc_pages;
  use Protection;

  #[test]
  fn lock_page() {
    let map = alloc_pages(&[Protection::READ_WRITE]);
    let _guard = lock(map.as_ptr(), page_size()).unwrap();
  }

  #[test]
  fn lock_page_release() {
    let map = alloc_pages(&[Protection::READ_WRITE]);

    unsafe {
      ::std::mem::forget(lock(map.as_ptr(), page_size()).unwrap());
      unlock(map.as_ptr(), page_size()).unwrap();
    }
  }
}
