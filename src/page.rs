//! Page related functions.

use crate::os;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Returns the operating system's page size.
///
/// This function uses an internally cached page size, and can be called
/// repeatedly without incurring a significant performance penalty.
///
/// # Examples
///
/// ```
/// # use region::page;
/// let size = page::size(); // Most likely 4096
/// ```
#[inline]
pub fn size() -> usize {
  static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

  match PAGE_SIZE.load(Ordering::Relaxed) {
    0 => {
      let size = os::page_size();
      PAGE_SIZE.store(size, Ordering::Relaxed);
      size
    }
    size => size,
  }
}

/// Rounds an address down to its closest page boundary.
///
/// # Examples
///
/// ```
/// # use region::page;
/// let unaligned_pointer = (page::size() + 1) as *const ();
///
/// assert_eq!(page::floor(unaligned_pointer), page::size() as *const _);
/// ```
#[inline]
pub fn floor<T>(address: *const T) -> *const T {
  let address = address.addr() & !(size() - 1);
  core::ptr::with_exposed_provenance(address)
}

/// Rounds an address up to its closest page boundary.
///
/// # Examples
///
/// ```
/// # use region::page;
/// let unaligned_pointer = (page::size() - 1) as *const ();
///
/// assert_eq!(page::ceil(unaligned_pointer), page::size() as *const _);
/// ```
#[inline]
pub fn ceil<T>(address: *const T) -> *const T {
  match address.addr().checked_add(size()) {
    Some(offset) => {
      let address = (offset - 1) & !(size() - 1);
      core::ptr::with_exposed_provenance(address)
    }
    None => floor(address),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn page_size_is_reasonable() {
    let pz = size();

    assert!(pz > 0);
    assert_eq!(pz % 2, 0);
    assert_eq!(pz, size());
  }

  #[test]
  fn page_rounding_works() {
    let pz = size();
    let point = core::ptr::without_provenance::<()>(1);

    assert_eq!(floor(point).addr(), 0);
    assert_eq!(floor(core::ptr::without_provenance::<()>(pz)).addr(), pz);
    assert_eq!(
      floor(core::ptr::without_provenance::<()>(usize::MAX)).addr() % pz,
      0
    );

    assert_eq!(ceil(point).addr(), pz);
    assert_eq!(ceil(core::ptr::without_provenance::<()>(pz)).addr(), pz);
    assert_eq!(
      ceil(core::ptr::without_provenance::<()>(usize::MAX)).addr() % pz,
      0
    );
  }
}
