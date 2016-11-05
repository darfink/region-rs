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

//pub fn lock(address: *const u8, size: usize) -> Result<()> { }
//pub fn unlock(address: *const u8, size: usize) -> Result<()> { }

pub fn query(address: *const u8) -> Result<Region, Error> {
    if address.is_null() {
        return Err(Error::Null);
    }

    // The address must be aligned to the closest page boundary
    os::get_region(os::page_floor(address as usize) as *const u8)
}

pub fn query_area(address: *const u8, size: usize) -> Result<Vec<Region>, Error> {
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
    os::set_protection(base as *const u8, size, protection)
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
    fn query_area_zero() {
        let region = query_area(&query_area_zero as *const _ as *const u8, 0).unwrap();
        assert_eq!(region.len(), 1);
    }

    #[test]
    fn query_area_overlap() {
        let pz = ::os::page_size();
        let prots = [Protection::ReadExecute, Protection::ReadWrite];
        let map = alloc_pages(&prots);

        let address = unsafe { map.ptr().offset(pz as isize - 1) };
        let result = query_area(address, 2).unwrap();

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
        let result = query_area(map.ptr(), pz).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].protection, prots[0]);

        // Retrieve all allocated pages
        let result = query_area(map.ptr(), pz * prots.len()).unwrap();
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
        let size = 0x100;
        let mut map = Mmap::anonymous(size, memmap::Protection::Read).unwrap();

        protect(map.ptr(), size, Protection::ReadWrite).unwrap();
        unsafe {
            *map.mut_ptr() = 0x1;
        }
    }

    #[test]
    fn alloc_query_protect() {
        let page_size = ::os::page_size();
        let prots = [Protection::Read, Protection::ReadWrite];
        let mut map = alloc_pages(&prots);

        // Validate the properties of the first page
        let region = query(map.ptr()).unwrap();
        assert_eq!(region.protection, prots[0]);
        assert!(!region.base.is_null() && region.base <= map.mut_ptr());
        assert!(region.size >= page_size);

        // Assert that the adjacent page has the new properties
        let adjacent_page = unsafe { map.ptr().offset(page_size as isize) };
        let region = query(adjacent_page).unwrap();

        assert_eq!(region.base as *const u8, adjacent_page);
        assert_eq!(region.protection, prots[1]);
        assert!(region.size >= page_size);
    }
}
