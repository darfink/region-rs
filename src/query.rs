use crate::{os, round_to_page_boundaries, Error, Region, Result};

/// Queries the OS with an address, returning the region it resides within.
///
/// If the queried address does not reside within any mapped region,
/// [Error::UnmappedRegion] will be returned.
///
/// # Parameters
///
/// - The enclosing region can be of multiple page sizes.
/// - The address is rounded down to the closest page boundary.
///
/// # Windows
///
/// Memory pages that are `MEM_RESERVE` or `MEM_FREE` are discarded. To ensure a
/// consistent cross-platform behavior, both types are represented as
/// [Error::UnmappedRegion].
///
/// On Windows, in contrast to other operating systems, a region does not include
/// pages with the same properties *before* the provided `address`. This is due
/// to the implemented behavior of `VirtualQuery`.
///
/// # Examples
///
/// ```
/// # fn main() -> region::Result<()> {
/// use region::Protection;
///
/// let data = [0; 100];
/// let region = region::query(data.as_ptr())?;
///
/// assert_eq!(region.protection(), Protection::READ_WRITE);
/// # Ok(())
/// # }
/// ```
pub fn query<T>(address: *const T) -> Result<Region> {
  // For UNIX systems, the address must be aligned to the closest page boundary
  let (address, size) = round_to_page_boundaries(address, 1)?;

  os::query(address, size)?
    .next()
    .ok_or(Error::UnmappedRegion)?
}

/// Queries the OS for mapped regions that overlap with the specified range.
///
/// In contrast to [query], this function only returns mapped regions. If
/// necessary, unmapped regions can be manually identified by inspecting
/// potential gaps between two neighboring regions.
///
/// If an error is encountered during iteration, the error will be the last item
/// that is yielded. Thereafter the iterator becomes fused.
///
/// A 2-byte range straddling a page boundary, will return both pages (or one
/// region, if the pages share the same properties).
///
/// # Parameters
///
/// - The range is `[address, address + size)`
/// - The address is rounded down to the closest page boundary.
/// - The size may not be zero.
///
/// # Examples
///
/// ```
/// # use region::Result;
/// # fn main() -> Result<()> {
/// let data = [0; 100];
/// let region = region::query_range(data.as_ptr(), data.len())?
///   .collect::<Result<Vec<_>>>()?;
///
/// assert_eq!(region.len(), 1);
/// assert_eq!(region[0].protection(), region::Protection::READ_WRITE);
/// # Ok(())
/// # }
/// ```
pub fn query_range<T>(
  address: *const T,
  size: usize,
) -> Result<impl Iterator<Item = Result<Region>>> {
  let (address, size) = round_to_page_boundaries(address, size)?;
  os::query(address, size)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::alloc_pages;
  use crate::{page, Protection};

  #[test]
  fn query_returns_unmapped_for_oob_address() {
    let (min, max) = (0 as *const (), usize::max_value() as *const ());
    assert!(matches!(query(min), Err(Error::UnmappedRegion)));
    assert!(matches!(query(max), Err(Error::UnmappedRegion)));
  }

  #[test]
  fn query_returns_correct_region_for_text_segment() -> Result<()> {
    println!(
      "{:?}",
      query(query_returns_correct_region_for_text_segment as *const ())
    );

    let region = query(query_returns_correct_region_for_text_segment as *const ())?;
    assert_eq!(region.protection(), Protection::READ_EXECUTE);
    assert_eq!(region.is_guarded(), false);
    assert_eq!(region.is_shared(), cfg!(windows));
    Ok(())
  }

  #[test]
  fn query_returns_one_region_for_identical_adjacent_pages() -> Result<()> {
    let pages = [Protection::READ_EXECUTE, Protection::READ_EXECUTE];
    let map = alloc_pages(&pages);
    let region = query(map.as_ptr())?;

    assert_eq!(region.is_guarded(), false);
    assert_eq!(region.protection(), Protection::READ_EXECUTE);
    assert!(region.as_ptr() <= map.as_ptr());
    assert!(region.len() >= page::size() * pages.len());
    Ok(())
  }

  #[test]
  fn query_is_not_off_by_one() -> Result<()> {
    let pages = [Protection::READ, Protection::READ_EXECUTE, Protection::READ];
    let map = alloc_pages(&pages);

    let page_mid = unsafe { map.as_ptr().offset(page::size() as isize) };
    let region = query(page_mid)?;

    assert_eq!(region.protection(), Protection::READ_EXECUTE);
    assert_eq!(region.len(), page::size());

    let region = query(unsafe { page_mid.offset(-1) })?;

    assert_eq!(region.protection(), Protection::READ);
    assert!(region.len() >= page::size());
    Ok(())
  }

  #[test]
  fn query_range_does_not_return_unmapped_regions() -> Result<()> {
    let regions = query_range(0 as *const (), 1)?.collect::<Result<Vec<_>>>()?;
    assert!(regions.is_empty());
    Ok(())
  }

  #[test]
  fn query_range_returns_both_regions_for_straddling_range() -> Result<()> {
    let pages = [Protection::READ_EXECUTE, Protection::READ_WRITE];
    let map = alloc_pages(&pages);

    // Query an area that overlaps both pages
    let address = unsafe { map.as_ptr().offset(page::size() as isize - 1) };
    let regions = query_range(address, 2)?.collect::<Result<Vec<_>>>()?;

    assert_eq!(regions.len(), pages.len());
    for (page, region) in pages.iter().zip(regions.iter()) {
      assert_eq!(*page, region.protection);
    }
    Ok(())
  }

  #[test]
  fn query_range_has_inclusive_lower_and_exclusive_upper_bound() -> Result<()> {
    let pages = [Protection::READ, Protection::READ_WRITE, Protection::READ];
    let map = alloc_pages(&pages);

    let regions = query_range(map.as_ptr(), page::size())?.collect::<Result<Vec<_>>>()?;
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].protection(), Protection::READ);

    let regions = query_range(map.as_ptr(), page::size() + 1)?.collect::<Result<Vec<_>>>()?;
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].protection(), Protection::READ);
    assert_eq!(regions[1].protection(), Protection::READ_WRITE);
    Ok(())
  }

  #[test]
  fn query_range_can_iterate_over_entire_process() -> Result<()> {
    let regions = query_range(0 as *const (), usize::max_value())?.collect::<Result<Vec<_>>>()?;
    let (r, rw, rx) = (
      Protection::READ,
      Protection::READ_WRITE,
      Protection::READ_EXECUTE,
    );

    // This test is a bit rough around the edges
    assert!(regions.iter().any(|region| region.protection() == r));
    assert!(regions.iter().any(|region| region.protection() == rw));
    assert!(regions.iter().any(|region| region.protection() == rx));
    assert!(regions.len() > 5);
    Ok(())
  }
}
