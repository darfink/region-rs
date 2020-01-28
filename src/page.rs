//! Page related functions.

use os;
use std::sync::Once;

/// Returns the operating system's page size.
///
/// This call internally caches the page size and can therefore be called
/// frequently without any performance penalty.
#[inline]
pub fn size() -> usize {
  static INIT: Once = Once::new();
  static mut PAGE_SIZE: usize = 0;

  unsafe {
    INIT.call_once(|| PAGE_SIZE = os::page_size());
    PAGE_SIZE
  }
}

/// Rounds an address down to the closest page boundary.
#[inline]
pub fn floor(address: usize) -> usize {
  address & !(size() - 1)
}

/// Rounds an address up to the closest page boundary.
#[inline]
pub fn ceil(address: usize) -> usize {
  let page_size = size();
  (address + page_size - 1) & !(page_size - 1)
}

/// Rounds a size up to the closest page boundary, relative to an address.
#[inline]
pub fn size_from_range(address: *const u8, sz: usize) -> usize {
  let sz = if sz == 0 { size() } else { sz };

  // The [address+size] may straddle between two or more pages; e.g if the
  // address is 4095 and the size is 2, this may be rounded up to 8192.
  ceil(address as usize % size() + sz)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn page_size_value() {
    let pz = size();

    assert!(pz > 0);
    assert!(pz % 2 == 0);
  }

  #[test]
  fn page_rounding() {
    let pz = size();

    // Truncates down
    assert_eq!(floor(1), 0);
    assert_eq!(floor(pz), pz);
    assert_eq!(floor(pz + 1), pz);

    // Rounds up
    assert_eq!(ceil(0), 0);
    assert_eq!(ceil(1), pz);
    assert_eq!(ceil(pz), pz);
    assert_eq!(ceil(pz + 1), pz * 2);
  }
}
