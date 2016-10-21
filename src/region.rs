extern crate regex;

use {std, os, Error, Protection, Access};
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct Query {
    base: *mut u8,
    size: usize,
    protection: Protection,
    guarded: bool,
}

#[derive(Debug)]
pub struct Page {
    pub size: usize,
    pub base: *mut u8,
    pub guarded: bool,
    pub protection: Protection,
    previous_protection: Protection,
    initial_protection: Protection,
}

#[derive(Debug)]
pub struct Region {
    pages: Vec<Page>,
    range: std::ops::Range<usize>,
}

impl Region {
    pub fn new(address: *const u8) -> Result<Self> {
        Self::with_size_impl(address, None)
    }

    pub fn with_size(address: *const u8, size: usize) -> Result<Self> {
        Self::with_size_impl(address, Some(size))
    }

    pub fn exec_with_prot<T: FnOnce()>(&mut self, protection: Protection, callback: T) -> Result<()> {
        try!(self.set_prot(Access::Type(protection)));
        callback();
        try!(self.set_prot(Access::Previous));
        Ok(())
    }

    pub fn set_prot(&mut self, flag: Access) -> Result<()> {
        for page in &mut self.pages {
            let protection = match flag {
                Access::Initial => page.initial_protection,
                Access::Previous => page.previous_protection,
                Access::Type(protection) => protection,
            };

            page.previous_protection = try!(Self::set_page_prot(&page, protection));
            page.protection = protection;
        }

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        self.pages.clear();
        let page_size = os::page_size();

        // Iterate over one page at a time
        let mut iter = self.range.clone().step_by(page_size);

        // Iterator is mutated inside the loop, so use 'while' instead of 'for'
        while let Some(page_base) = iter.next() {
            // When a memory region is queried, the size is equal to all memory
            // pages that lie consecutively in memory with the same flags. To
            // avoid unnecessary API calls, one might be enough to retrieve the
            // memory information for all pages within the address range.
            let region = try!(Self::query(page_base as *const u8));

            // Number of pages required to cover the specified address range
            let pages_left = self.pages.capacity() - self.pages.len();

            // Number of pages left in the region, relative to the current base address
            let pages_range = (region.base as usize + region.size - page_base) / page_size;

            for page in 0..std::cmp::min(pages_left, pages_range) {
                // Add each page from the region
                self.pages.push(Page {
                    size: page_size,
                    base: (page_base + page * page_size) as *mut u8,
                    guarded: region.guarded,
                    protection: region.protection,
                    previous_protection: region.protection,
                    initial_protection: region.protection,
                });

                if page > 0 {
                    // When more than one page is added per query, the iterator
                    // needs to advance to the consecutive page boundary.
                    iter.next();
                }
            }
        }

        Ok(())
    }

    fn with_size_impl(address: *const u8, size: Option<usize>) -> Result<Self> {
        if address.is_null() {
            return Err(Error::Null);
        }

        let size = match size {
            Some(size) => size,
            None => {
                // Lazily implement this by using an otherwise redundant OS call
                // leading to much simpler code at the cost of performance.
                let region = try!(Self::query(address));

                // Create a range from the address to the end of its enclosing region
                (region.base as usize + region.size) - address as usize
            }
        };

        // Align the address to page boundaries, where 'start' is the
        // closest lower page, and 'end' is the closest upper page.
        let range = os::truncate_page(address as usize)..os::round_page(address as usize + size);

        let mut region = Region {
            pages: Vec::with_capacity(range.len() / os::page_size()),
            range: range,
        };

        try!(region.update());
        Ok(region)
    }

}


#[cfg(unix)]
impl Region {
    fn set_page_prot(page: &Page, protection: Protection) -> Result<Protection> {
        let result = unsafe {
            ::libc::mprotect(page.base as *mut ::libc::c_void, page.size, protection.into())
        };

        match result {
            0 => Ok(page.previous_protection),
            _ => Err(Error::Mprotect(::errno::errno()))
        }
    }

    #[cfg(target_os = "linux")]
    fn query(address: *const u8) -> Result<Query> {
        use std::fs::File;
        use std::io::{BufReader, BufRead};
        use self::regex::Regex;

        lazy_static! {
            static ref RE: Regex = Regex::new("^([0-9a-fA-F]+)-([0-9a-fA-F]+) (\\w|-){4}").unwrap();
        }

        let address = address as usize;
        let file = try!(File::open("/proc/self/maps").map_err(Error::ProcfsIo));
        let reader = BufReader::new(&file).lines();

        for line in reader {
            let line = try!(line.map_err(Error::ProcfsIo));
            if let Some(captures) = RE.captures(&line) && captures.len() == 3 {
                let lower = try!(usize::from_str_radix(captures.at(0), 16).map_err(Error::ProcsfsParse));
                let upper = try!(usize::from_str_radix(captures.at(1), 16).map_err(Error::ProcsfsParse));

                if address >= lower && address < upper {
                    let protection = captures.at(2).unwrap();
                    return Ok(QueryInfo {
                        base: lower as *mut u8,
                        size: upper - lower,
                        protection: protection.into(),
                        guarded: false,
                    });
                }
            } else {
                return Err(Error::ProcfsMatches);
            }
        }

        Err(Error::ProcfsRange)
    }

    #[cfg(target_os = "macos")]
    fn query(address: *const u8) -> Result<Query> {
        extern crate mach;

        // The address is aligned to the enclosing region
        let mut region_base = address as mach::vm_types::mach_vm_address_t;
        let mut region_size: mach::vm_types::mach_vm_size_t = 0;
        let mut info: mach::vm_region::vm_region_extended_info = unsafe { std::mem::zeroed() };

        let result = unsafe {
            // This information is of no interest
            let mut object_name: mach::port::mach_port_t = 0;

            // Query the OS about the memory region
            mach::vm::mach_vm_region(
                mach::traps::mach_task_self(),
                &mut region_base,
                &mut region_size,
                mach::vm_region::VM_REGION_EXTENDED_INFO,
                (&mut info as *mut _) as mach::vm_region::vm_region_info_t,
                &mut mach::vm_region::vm_region_extended_info::count(),
                &mut object_name)
        };

        match result {
            mach::kern_return::KERN_SUCCESS => Ok(Query {
                base: region_base as *mut u8,
                size: region_size as usize,
                protection: info.protection.into(),
                guarded: false,//(info.user_tag == mach::vm_statistics::VM_MEMORY_GUARD),
            }),
            _ => Err(Error::MachRegion(result)),
        }
    }
}

#[cfg(windows)]
impl Region {
    fn set_page_prot(page: &Page, protection: Protection) -> Result<Protection> {
        let mut prev_flags = 0;
        let result = unsafe {
            winapi::VirtualProtect(page.base as winapi::PVOID, page.size, protection.into(), &mut prev_flags);
        };

        match result {
            winapi::ERROR_SUCCESS => Ok(prev_flags.into()),
            _ => Err(Error::VirtualProtect(::errno::Errno(result))),
        }
    }

    fn query(address: *const u8) -> Result<Query> {
        extern crate winapi;

        let mut info: winapi::MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        let result = unsafe { winapi::VirtualQuery(address, &mut info, std::mem::size_of::<winapi::MEMORY_BASIC_INFORMATION>()) };

        match result {
            winapi::ERROR_SUCCESS => Ok(Query {
                base: info.BaseAddress,
                size: info.RegionSize,
                protection: info.Protect.into(),
                guarded: (info.Protect & winapi::PAGE_GUARD) != 0,
            }),
            _ => Err(Error::VirtualQuery(::errno::Errno(result)))
        }
    }
}
