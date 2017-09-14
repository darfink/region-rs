//! Page related functions.

use std::sync::{Once, ONCE_INIT};
use os;

/// Returns the operating system's current page size.
pub fn page_size() -> usize {
    static INIT: Once = ONCE_INIT;
    static mut PAGE_SIZE: usize = 0;

    unsafe {
        INIT.call_once(|| PAGE_SIZE = os::page_size());
        PAGE_SIZE
    }
}

/// Rounds an address down to the closest page boundary.
pub fn page_floor(address: usize) -> usize {
    address & !(page_size() - 1)
}

/// Rounds an address up to the closest page boundary.
pub fn page_ceil(address: usize) -> usize {
    let page_size = page_size();
    (address + page_size - 1) & !(page_size - 1)
}

/// Rounds a size up to the closest page boundary, relative to an address.
pub fn page_size_from_range(address: *const u8, size: usize) -> usize {
    let size = if size == 0 { page_size() } else { size };

    // The [address+size] may straddle between two or more pages; e.g if the
    // address is 4095 and the size is 2 this will be rounded up to 8192 (on
    // x86).
    page_ceil(address as usize % page_size() + size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_value() {
        let pz = page_size();

        assert!(pz > 0);
        assert!(pz % 2 == 0);
    }

    #[test]
    fn page_rounding() {
        let pz = page_size();

        // Truncates down
        assert_eq!(page_floor(1), 0);
        assert_eq!(page_floor(pz), pz);
        assert_eq!(page_floor(pz + 1), pz);

        // Rounds up
        assert_eq!(page_ceil(0), 0);
        assert_eq!(page_ceil(1), pz);
        assert_eq!(page_ceil(pz), pz);
        assert_eq!(page_ceil(pz + 1), pz * 2);
    }
}
