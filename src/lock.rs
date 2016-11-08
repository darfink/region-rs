use Error;
use os;

/// Locks one or more memory regions to RAM.
///
/// The memory pages within the address range is guaranteed to stay in RAM
/// except for specials cases such as hibernation and memory starvation.
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Examples
///
/// ```
/// let data = [0; 100];
/// let _guard = region::lock(data.as_ptr(), data.len()).unwrap();
/// ```
pub fn lock(address: *const u8, size: usize) -> Result<LockGuard, Error> {
    try!(os::lock(os::page_floor(address as usize) as *const u8,
                  os::page_size_from_range(address, size)));
    Ok(LockGuard::new(address, size))
}

/// Unlocks one or more memory regions from RAM.
///
/// This function is unsafe since it cannot be known whether it is called on a
/// locked region or not. In normal uses cases, the `LockGuard` is recommended
/// for safe code.
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
pub unsafe fn unlock(address: *const u8, size: usize) -> Result<(), Error> {
    os::unlock(os::page_floor(address as usize) as *const u8,
               os::page_size_from_range(address, size))
}

/// An RAII implementation of a "scoped lock". When this structure is dropped
/// (falls out of scope), the virtual lock will be unlocked.
pub struct LockGuard {
    address: *const u8,
    size: usize,
    free: bool,
}

impl LockGuard {
    fn new(address: *const u8, size: usize) -> Self {
        LockGuard {
            address: address,
            size: size,
            free: true,
        }
    }

    /// Releases the guards ownership of the virtual lock.
    pub unsafe fn release(mut self) {
        self.free = false;
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if self.free {
            assert!(unsafe { ::unlock(self.address, self.size).is_ok() });
        }
    }
}

#[cfg(test)]
mod tests {
    use Protection;
    use os::page_size;
    use super::*;
    use tests::alloc_pages;

    #[test]
    fn lock_page() {
        let map = alloc_pages(&[Protection::ReadWrite]);
        let _guard = lock(map.ptr(), page_size()).unwrap();
    }

    #[test]
    fn lock_page_release() {
        let map = alloc_pages(&[Protection::ReadWrite]);

        unsafe {
            lock(map.ptr(), page_size()).unwrap().release();
            unlock(map.ptr(), page_size()).unwrap();
        }
    }

}
