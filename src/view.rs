use std::mem;
use error::*;
use {
    page,
    protect,
    query_range,
    lock,
    Protection,
    Region
};

// This is needed to preserve previous and initial protection values
#[derive(Debug, Copy, Clone)]
struct RegionMeta {
    region: Region,
    previous: Protection,
    initial: Protection,
}

/// View protection access.
#[derive(Debug, Copy, Clone)]
pub enum Access {
    /// A new protection state.
    Type(Protection),
    /// The previous protection state.
    Previous,
    /// The initial protection state.
    Initial,
}

impl From<Protection> for Access {
    fn from(protection: Protection) -> Access {
        Access::Type(protection)
    }
}

/// A view aligned to page boundaries.
///
/// This view is not aligned to regions, but uses the same granularity as the
/// OSs page size. Therefore it is useful when changing protection, only
/// affecting the pages within the view (instead of entire regions).
///
/// Beyond this, it also preserves the previous and initial protection state of
/// all pages within the view. This allows for easily changing states while,
/// still being able to restore them at a later stage.
///
/// # Implementation
///
/// Consider the following 6 pages in memory:
///
/// ```c
/// 0      4096     8192     12288    16384    20480    24576
/// +--------+--------+--------+--------+--------+--------+
/// |  4096  |  4096  |  4096  |  4096  |  4096  |  4096  |
/// |   RW   |   RX   |   RX   |   RX   |   RX   |   RW   |
/// +--------+--------+--------+--------+--------+--------+
///          |    Region (RX), size of 16 384    |
///          +--------+-----------------+--------+
///                   | View (len 8192) |
///                   +-----------------+
/// ```
///
/// If the view is created with the values:
///
/// - `address = 8500` (rounded down to the closest page boundary).
/// - `size = 4000` (rounded up to the closest page boundary, relative to the
///   address).
///
/// It will contain all pages within the range `[8192, 16384)`, and have a
/// `len()` of `8192`, ignoring the boundaries of the intersecting region.
///
/// # Examples
///
/// ```
/// use region::{View, Protection};
///
/// let ret5 = [0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
/// let mut view = View::new(ret5.as_ptr(), ret5.len()).unwrap();
///
/// unsafe {
///   view.exec_with_prot(Protection::ReadWriteExecute, || {
///     let x: extern "C" fn() -> i32 = unsafe { std::mem::transmute(ret5.as_ptr()) };
///     assert_eq!(x(), 5);
///   }).unwrap()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct View {
    regions: Vec<RegionMeta>,
}

impl View {
    /// Constructs a new page view.
    ///
    /// The constructor uses `query_range` internally.
    ///
    /// - The address is aligned to the closest page boundary.
    /// - The upper bound (`address + size`) is rounded up to the closest page
    ///   boundary.
    pub fn new(address: *const u8, size: usize) -> Result<Self> {
        let mut regions = query_range(address, size)?;

        let lower = page::page_floor(address as usize);
        let upper = page::page_ceil(address as usize + size);

        if let Some(ref mut region) = regions.first_mut() {
            // Offset the lower region to the smallest page boundary
            let delta = lower - region.base as usize;
            region.base = (region.base as usize + delta) as *mut u8;
            region.size -= delta;
        }

        if let Some(ref mut region) = regions.last_mut() {
            // Truncate the upper region to the smallest page boundary
            let delta = region.upper() - upper;
            region.size -= delta;
        }

        Ok(View {
            regions: regions.iter()
                .map(|region| RegionMeta {
                    region: *region,
                    previous: region.protection,
                    initial: region.protection,
                })
                .collect::<Vec<_>>(),
        })
    }

    /// Returns the protection of the pages within the view.
    ///
    /// This will only return the protection if all containing pages have the
    /// same protection, otherwise `None` will be returned.
    pub fn get_prot(&self) -> Option<Protection> {
        let prot = self.regions.iter()
            .fold(Protection::None, |prot, meta| prot | meta.region.protection);

        if prot == self.regions.first().unwrap().region.protection {
            Some(prot)
        } else {
            None
        }
    }

    /// Sets the protection of all pages within the view.
    ///
    /// Besides applying a new protection state to all pages within the view,
    /// this function can also reset the protection of all pages to their
    /// initial state (`Access::Initial`) or their previous state
    /// (`Access::Previous`).
    ///
    /// If an access value besides `Access::Type` is used, it may result in
    /// multiple OS calls depending on the number of pages.
    pub unsafe fn set_prot<A: Into<Access>>(&mut self, access: A) -> Result<()> {
        match access.into() {
            Access::Type(protection) => {
                // Alter the protection of the whole view at once
                protect(self.as_ptr(), self.len(), protection)?;

                for meta in &mut self.regions {
                    // Update the current and previous protection flags
                    meta.previous = meta.region.protection;
                    meta.region.protection = protection;
                }
            }
            Access::Previous => {
                for meta in &mut self.regions {
                    protect(meta.region.base, meta.region.size, meta.previous)?;
                    mem::swap(&mut meta.region.protection, &mut meta.previous);
                }
            }
            Access::Initial => {
                for meta in &mut self.regions {
                    protect(meta.region.base, meta.region.size, meta.initial)?;
                    meta.previous = meta.region.protection;
                    meta.region.protection = meta.initial;
                }
            }
        }

        Ok(())
    }

    /// Executes a closure while temporarily changing protection state.
    ///
    /// This is a comfortable shorthand method, functionally equivalent to
    /// calling `set_prot(prot.into())`, executing arbitrary code, followed by
    /// `set_prot(Access::Previous)`.
    pub unsafe fn exec_with_prot<Ret, T: FnOnce() -> Ret>(
            &mut self,
            prot: Protection,
            callback: T)
            -> Result<Ret> {
        self.set_prot(prot)?;
        let result = callback();
        self.set_prot(Access::Previous)?;
        Ok(result)
    }

    /// Locks all memory pages within the view.
    ///
    /// The view itself does not do any bookkeeping related to whether the pages
    /// are locked or not (since the state can change from outside the library).
    pub fn lock(&mut self) -> Result<::LockGuard> {
        lock(self.as_ptr(), self.len())
    }

    /// Returns the view's base address.
    pub fn as_ptr(&self) -> *const u8 {
        self.regions.first().unwrap().region.base
    }

    /// Returns the view's base address as mutable
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.regions.first().unwrap().region.base as *mut _
    }

    /// Returns the view's lower bound.
    pub fn lower(&self) -> usize {
        self.regions.first().unwrap().region.lower()
    }

    /// Returns the view's upper bound.
    pub fn upper(&self) -> usize {
        self.regions.last().unwrap().region.upper()
    }

    /// Returns the length of the view.
    pub fn len(&self) -> usize {
        self.upper() - self.lower()
    }

    /// Returns whether this view is empty or not.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use Protection;
    use tests::alloc_pages;
    use super::*;

    #[test]
    fn view_check_size() {
        let pz = page::page_size();
        let map = alloc_pages(&[Protection::Read, Protection::Read, Protection::Read]);

        // Ensure that only one page is in the view
        let base = unsafe { map.as_ptr().offset(pz as isize) };
        let view = View::new(base, pz).unwrap();
        assert_eq!(view.as_ptr(), base);
        assert_eq!(view.len(), pz);

        // Ensure that two pages are in the view (when straddling on page boundary)
        let base = unsafe { map.as_ptr().offset(pz as isize - 1) };
        let view = View::new(base, 2).unwrap();
        assert_eq!(view.as_ptr(), map.as_ptr());
        assert_eq!(view.len(), pz * 2);
    }

    #[test]
    fn view_exec_prot() {
        let pz = page::page_size();
        let mut map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.as_ptr(), pz).unwrap();
        unsafe {
            let val = view.exec_with_prot(Protection::ReadWrite, || {
                *map.as_mut_ptr() = 0x10;
                1337
            }).unwrap();
            assert_eq!(val, 1337);
        }

        // Ensure that the protection returned to its previous state
        let region = ::query(view.as_ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_prot_prev() {
        let pz = page::page_size();
        let map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.as_ptr(), pz).unwrap();
        unsafe {
            view.set_prot(Protection::ReadWrite).unwrap();
            view.set_prot(Access::Previous).unwrap();
        }

        let region = ::query(view.as_ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_prot_initial() {
        let pz = page::page_size();
        let map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.as_ptr(), pz).unwrap();
        unsafe {
            view.set_prot(Protection::ReadWrite).unwrap();
            view.set_prot(Protection::ReadWriteExecute).unwrap();
            view.set_prot(Access::Initial).unwrap();
        }

        let region = ::query(view.as_ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_get_prot() {
        let pz = page::page_size();
        let map = alloc_pages(&[Protection::Read, Protection::ReadWrite]);

        let mut view = View::new(map.as_ptr(), pz * 2).unwrap();
        assert_eq!(view.len(), pz * 2);
        assert_eq!(view.get_prot(), None);

        unsafe { view.set_prot(Protection::Read).unwrap() };
        assert_eq!(view.get_prot(), Some(Protection::Read));
    }
}
