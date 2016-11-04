#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate lazy_static;
extern crate errno;
extern crate libc;

pub use error::Error;
pub use protection::Protection;
pub use region::Region;

mod error;
mod os;
mod protection;
mod region;

pub fn query(address: *const u8) -> Result<Region, Error> {
    if address.is_null() {
        return Err(Error::Null);
    }

    // The address must be aligned to the closest page boundary
    os::get_region(os::page_floor(address as usize) as *const u8)
}

pub fn protect(address: *const u8, size: usize, protection: Protection::Flag) -> Result<(), Error> {
    if address.is_null() {
        return Err(Error::Null);
    }

    let size = if size == 0 {
        os::page_size()
    } else {
        size
    };

    // The address must be aligned to the closest page boundary
    let base = os::page_floor(address as usize);

    // The [address+size] may straddle between two or more pages; e.g if the
    // address is 4095 and the size is 2 this will be rounded up to 8192 (on
    // x86). Therefore more than one page may be affected by this call.
    let size = os::page_ceil((address as usize) % os::page_size() + size);

    // Ignore the preservation of previous protection flags
    os::set_prot(base as *const u8, size, protection)
}

#[cfg(test)]
mod tests {
    extern crate memmap;

    use self::memmap::Mmap;
    use super::*;

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
        let mut map = Mmap::anonymous(size, memmap::Protection::ReadExecute).unwrap();
        let region = query(map.ptr()).unwrap();

        assert_eq!(region.guarded, false);
        assert_eq!(region.protection, Protection::ReadExecute);
        assert!(!region.base.is_null() && region.base <= map.mut_ptr());
        assert!(region.size >= size);
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
        let size = 0x100;
        let mut map = Mmap::anonymous(size, memmap::Protection::Read).unwrap();

        protect(map.ptr(), size, Protection::ReadWrite).unwrap();
        unsafe {
            *map.mut_ptr() = 0x1;
        }
    }

    #[test]
    fn alloc_query_protect() {
        let pz = ::os::page_size();
        let size = pz * 2;

        // Allocate memory that is at least two pages
        let mut map = Mmap::anonymous(size, memmap::Protection::Read).unwrap();

        // Validate the properties of the allocated memory
        let region = query(map.ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
        assert!(!region.base.is_null() && region.base <= map.mut_ptr());
        assert!(region.size >= size);

        // Update the protection flags of the adjacent page
        let adjacent_page = unsafe { map.mut_ptr().offset(pz as isize) };
        protect(adjacent_page, pz, Protection::ReadWrite).unwrap();

        // Assert that the adjacent page has the new properties
        let region = query(adjacent_page).unwrap();
        assert_eq!(region.base, adjacent_page);
        assert_eq!(region.protection, Protection::ReadWrite);
        assert!(region.size >= pz);

        // Ensure the first page retain the same properties
        let region = query(map.ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }
}
