//! A library for manipulating memory regions
//!
//! This crate provides several functions for handling memory pages and regions.
//! It is implemented using platform specific APIs. The library is relatively
//! bare metal.
//!
//! Not all OS specific quirks are abstracted away. For instance; some OSs
//! enforce memory pages to be readable whilst other may prevent pages from
//! becoming executable (i.e DEP).
//!
//! # Usage
//!
//! This crate is [on crates.io](https://crates.io/crates/region) and can be
//! used by adding `region` to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! region = "0.0.1"
//! ```
//!
//! and this to your crate root:
//!
//! ```rust
//! extern crate region;
//! ```

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate lazy_static;
extern crate errno;
extern crate libc;

pub use os::page_size;
pub use error::Error;
pub use protection::Protection;
pub use region::Region;

mod error;
mod os;
mod protection;
mod region;

/// Locks memory regions to RAM.
///
/// The memory pages within the address range is guaranteed to stay in RAM
/// except for cases such as hibernation and memory starvation.
///
/// - The address is rounded down to the closest page boundary.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Examples
///
/// ```
/// let data = [0; 100];
/// region::lock(data.as_ptr(), data.len()).unwrap();
/// region::unlock(data.as_ptr(), data.len()).unwrap();
/// ```
pub fn lock(address: *const u8, size: usize) -> Result<(), Error> {
    os::lock(os::page_floor(address as usize) as *const u8,
             os::page_size_from_range(address, size))
}

/// Unlocks memory regions from RAM.
///
/// - The address is rounded down to the closest page boundary.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
pub fn unlock(address: *const u8, size: usize) -> Result<(), Error> {
    os::unlock(os::page_floor(address as usize) as *const u8,
               os::page_size_from_range(address, size))
}

/// Queries the OS with an address, returning the region it resides within.
///
/// The implementation uses `VirtualQuery` on Windows, `mach_vm_region` on macOS
/// and by parsing `proc/[pid]/maps` on Linux.
///
/// - The enclosing region can be of multiple page sizes.
/// - The address is rounded down to the closest page boundary.
/// - The address may not be null.
///
/// # Examples
///
/// ```
/// use region::{Protection};
///
/// let data = [0; 100];
/// let region = region::query(data.as_ptr()).unwrap();
///
/// assert_eq!(region.protection, Protection::ReadWrite);
/// ```
pub fn query(address: *const u8) -> Result<Region, Error> {
    if address.is_null() {
        return Err(Error::Null);
    }

    // The address must be aligned to the closest page boundary
    os::get_region(os::page_floor(address as usize) as *const u8)
}

/// Queries the OS with a range, returning the regions it contains.
///
/// The range is from `address` (inclusive) to `address + size` (exclusive).
/// Therefore a 2-byte range straddling a page boundary will return both pages
/// (or one region, if the pages have the same properties). The implementation
/// uses `query` internally.
///
/// - The address is rounded down to the closest page boundary.
/// - The address may not be null.
///
/// # Examples
///
/// ```
/// let data = [0; 100];
/// let region = region::query_range(data.as_ptr(), data.len()).unwrap();
///
/// assert!(region.len() > 0);
/// ```
pub fn query_range(address: *const u8, size: usize) -> Result<Vec<Region>, Error> {
    let mut result = Vec::new();
    let mut base = os::page_floor(address as usize);
    let limit = address as usize + size;

    loop {
        let region = try!(query(base as *const u8));
        result.push(region);
        base = region.upper();

        if limit <= region.upper() {
            break;
        }
    }

    Ok(result)
}

/// Changes the memory protection of one or more regions.
///
/// The address range may overlap one or more regions, and if so, all regions
/// within the range will be modified. The previous protection flags are not
/// preserved (to reset protection flags to their inital values, `query_range`
/// can be used prior to this call).
///
/// If the size is zero this will affect the whole page located at the address
///
/// - The address may not be null.
/// - The size is rounded up to the closest page boundary, relative to the
///   address.
///
/// # Examples
///
/// ```
/// use region::{Protection};
///
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
/// region::protect(ret5.as_ptr(), ret5.len(), Protection::ReadWriteExecute).unwrap();
///
/// let x: extern "C" fn() -> i32 = unsafe { std::mem::transmute(ret5.as_ptr()) };
/// assert_eq!(x(), 5);
/// ```
pub fn protect(address: *const u8, size: usize, protection: Protection::Flag) -> Result<(), Error> {
    if address.is_null() {
        return Err(Error::Null);
    }

    // Ignore the preservation of previous protection flags
    os::set_protection(os::page_floor(address as usize) as *const u8,
                       os::page_size_from_range(address, size),
                       protection)
}

#[cfg(test)]
mod tests {
    extern crate memmap;

    use self::memmap::Mmap;
    use super::*;

    fn alloc_pages(prots: &[Protection::Flag]) -> Mmap {
        let pz = ::os::page_size();
        let map = Mmap::anonymous(pz * prots.len(), memmap::Protection::Read).unwrap();
        let mut base = map.ptr();

        for protection in prots {
            protect(base, pz, *protection).unwrap();
            base = unsafe { base.offset(pz as isize) };
        }

        map
    }

    #[test]
    fn lock_page() {
        let map = alloc_pages(&[Protection::ReadWrite]);
        lock(map.ptr(), page_size()).unwrap();
        unlock(map.ptr(), page_size()).unwrap();
    }

    #[test]
    fn query_null() {
        assert!(query(::std::ptr::null()).is_err());
    }

    #[test]
    fn query_code() {
        let region = query(&query_code as *const _ as *const u8).unwrap();

        assert_eq!(region.guarded, false);
        assert_eq!(region.protection, Protection::ReadExecute);
        assert_eq!(region.shared, false);
    }

    #[test]
    fn query_alloc() {
        let size = ::os::page_size() * 2;
        let mut map = alloc_pages(&[Protection::ReadExecute, Protection::ReadExecute]);
        let region = query(map.ptr()).unwrap();

        assert_eq!(region.guarded, false);
        assert_eq!(region.protection, Protection::ReadExecute);
        assert!(!region.base.is_null() && region.base <= map.mut_ptr());
        assert!(region.size >= size);
    }

    #[test]
    fn query_area_zero() {
        let region = query_range(&query_area_zero as *const _ as *const u8, 0).unwrap();
        assert_eq!(region.len(), 1);
    }

    #[test]
    fn query_area_overlap() {
        let pz = ::os::page_size();
        let prots = [Protection::ReadExecute, Protection::ReadWrite];
        let map = alloc_pages(&prots);

        // Query an area that overlaps both pages
        let address = unsafe { map.ptr().offset(pz as isize - 1) };
        let result = query_range(address, 2).unwrap();

        assert_eq!(result.len(), prots.len());
        for i in 0..prots.len() {
            assert_eq!(result[i].protection, prots[i]);
        }
    }

    #[test]
    fn query_area_alloc() {
        let pz = ::os::page_size();
        let prots = [Protection::Read, Protection::ReadWrite, Protection::ReadExecute];
        let map = alloc_pages(&prots);

        // Confirm only one page is retrieved
        let result = query_range(map.ptr(), pz).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].protection, prots[0]);

        // Retrieve all allocated pages
        let result = query_range(map.ptr(), pz * prots.len()).unwrap();
        assert_eq!(result.len(), prots.len());
        assert_eq!(result[1].size, pz);
        for i in 0..prots.len() {
            assert_eq!(result[i].protection, prots[i]);
        }
    }

    #[test]
    fn protect_null() {
        assert!(protect(::std::ptr::null(), 0, Protection::None).is_err());
    }

    #[test]
    fn protect_code() {
        let address = &mut query_code as *mut _ as *mut u8;
        protect(address, 0x10, Protection::ReadWriteExecute).unwrap();
        unsafe {
            *address = 0x90;
        }
    }

    #[test]
    fn protect_alloc() {
        let mut map = alloc_pages(&[Protection::Read]);
        protect(map.ptr(), ::os::page_size(), Protection::ReadWrite).unwrap();
        unsafe {
            *map.mut_ptr() = 0x1;
        }
    }

    #[test]
    fn protect_overlap() {
        let pz = ::os::page_size();

        // Create a page boundary with different protection flags in the
        // upper and lower span, so the intermediate page sizes are fixed.
        let prots =
            [Protection::Read, Protection::ReadExecute, Protection::ReadWrite, Protection::Read];

        let map = alloc_pages(&prots);
        let base_exec = unsafe { map.ptr().offset(pz as isize) };
        let straddle = unsafe { base_exec.offset(pz as isize - 1) };

        // Change the protection over two page boundaries
        protect(straddle, 2, Protection::ReadWriteExecute).unwrap();

        // Ensure that the pages have merged into one region
        let result = query_range(base_exec, pz * 2).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].protection, Protection::ReadWriteExecute);
        assert_eq!(result[0].size, pz * 2);
    }
}
