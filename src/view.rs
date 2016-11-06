use Error;
use Protection;
use Region;
use os;

// This is needed to preserve previous and initial protection values
struct RegionMeta {
    region: Region,
    previous: Protection::Flag,
    initial: Protection::Flag,
}

/// View protection access.
pub enum Access {
    /// A new protection state.
    Type(Protection::Flag),
    /// The previous protection state.
    Previous,
    /// The initial protection state.
    Initial,
}

impl From<Protection::Flag> for Access {
    fn from(protection: Protection::Flag) -> Access {
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
/// view.exec_with_prot(Protection::ReadWriteExecute, || {
///   let x: extern "C" fn() -> i32 = unsafe { std::mem::transmute(ret5.as_ptr()) };
///   assert_eq!(x(), 5);
/// }).unwrap()
/// ```
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
    pub fn new(address: *const u8, size: usize) -> Result<Self, Error> {
        let mut regions = try!(::query_range(address, size));

        let lower = os::page_floor(address as usize);
        let upper = os::page_ceil(address as usize + size);

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
                .map(|region| {
                    RegionMeta {
                        region: *region,
                        previous: region.protection,
                        initial: region.protection,
                    }
                })
                .collect::<Vec<_>>(),
        })
    }

    /// Returns the protection of the pages within the view.
    ///
    /// This will only return the protection if all containing pages have the
    /// same protection, otherwise `None` will be returned.
    pub fn get_prot(&self) -> Option<Protection::Flag> {
        let prot = self.regions.iter().fold(Protection::None,
                                            |prot, ref meta| prot | meta.region.protection);

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
    pub fn set_prot(&mut self, access: Access) -> Result<(), Error> {
        match access {
            Access::Type(protection) => {
                // Alter the protection of the whole view at once
                try!(::protect(self.ptr(), self.len(), protection));

                for meta in &mut self.regions {
                    // Update the current and previous protection flags
                    meta.previous = meta.region.protection;
                    meta.region.protection = protection;
                }
            }
            Access::Previous => {
                for meta in &mut self.regions {
                    try!(::protect(meta.region.base, meta.region.size, meta.previous));
                    ::std::mem::swap(&mut meta.region.protection, &mut meta.previous);
                }
            }
            Access::Initial => {
                for meta in &mut self.regions {
                    try!(::protect(meta.region.base, meta.region.size, meta.initial));
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
    pub fn exec_with_prot<T: FnOnce()>(&mut self,
                                       prot: Protection::Flag,
                                       callback: T)
                                       -> Result<(), Error> {
        try!(self.set_prot(prot.into()));
        callback();
        try!(self.set_prot(Access::Previous));
        Ok(())
    }

    /// Locks all memory pages within the view.
    ///
    /// The view itself does not do any bookkeeping related to whether the pages
    /// are locked or not (since the state can change from outside the library).
    pub fn lock(&mut self) -> Result<::LockGuard, Error> {
        ::lock(self.ptr(), self.len())
    }

    /// Returns the view's base address.
    pub fn ptr(&self) -> *const u8 {
        self.regions.first().unwrap().region.base
    }

    /// Returns the view's base address as mutable
    pub fn mut_ptr(&mut self) -> *mut u8 {
        self.regions.first().unwrap().region.base
    }

    /// Returns the view's lower bound.
    pub fn lower(&self) -> usize {
        self.regions.first().unwrap().region.lower()
    }

    /// Returns the view's upper bound.
    pub fn upper(&self) -> usize {
        self.regions.last().unwrap().region.upper()
    }

    /// Returns the length of the view
    pub fn len(&self) -> usize {
        self.upper() - self.lower()
    }
}

#[cfg(test)]
mod tests {
    use Protection;
    use tests::alloc_pages;
    use os::page_size;
    use super::*;

    #[test]
    fn view_check_size() {
        let pz = page_size();
        let map = alloc_pages(&[Protection::Read, Protection::Read, Protection::Read]);

        // Ensure that only one page is in the view
        let base = unsafe { map.ptr().offset(pz as isize) };
        let view = View::new(base, pz).unwrap();
        assert_eq!(view.ptr(), base);
        assert_eq!(view.len(), pz);

        // Ensure that two pages are in the view (when straddling on page boundary)
        let base = unsafe { map.ptr().offset(pz as isize - 1) };
        let view = View::new(base, 2).unwrap();
        assert_eq!(view.ptr(), map.ptr());
        assert_eq!(view.len(), pz * 2);
    }

    #[test]
    fn view_exec_prot() {
        let pz = page_size();
        let mut map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.ptr(), pz).unwrap();
        view.exec_with_prot(Protection::ReadWrite, || unsafe {
                *map.mut_ptr() = 0x10;
            })
            .unwrap();

        // Ensure that the protection returned to its previous state
        let region = ::query(view.ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_prot_prev() {
        let pz = page_size();
        let map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.ptr(), pz).unwrap();
        view.set_prot(Protection::ReadWrite.into()).unwrap();
        view.set_prot(Access::Previous).unwrap();

        let region = ::query(view.ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_prot_initial() {
        let pz = page_size();
        let map = alloc_pages(&[Protection::Read]);

        let mut view = View::new(map.ptr(), pz).unwrap();
        view.set_prot(Protection::ReadWrite.into()).unwrap();
        view.set_prot(Protection::ReadWriteExecute.into()).unwrap();
        view.set_prot(Access::Initial).unwrap();

        let region = ::query(view.ptr()).unwrap();
        assert_eq!(region.protection, Protection::Read);
    }

    #[test]
    fn view_get_prot() {
        let pz = page_size();
        let map = alloc_pages(&[Protection::Read, Protection::ReadWrite]);

        let mut view = View::new(map.ptr(), pz * 2).unwrap();
        assert_eq!(view.len(), pz * 2);
        assert_eq!(view.get_prot(), None);

        view.set_prot(Protection::Read.into()).unwrap();
        assert_eq!(view.get_prot(), Some(Protection::Read));
    }
}
