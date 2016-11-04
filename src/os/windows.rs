extern crate winapi;

use Error;
use Protection;
use Region;

fn convert_to_native(protection: Protection) -> winapi::DWORD {
    match protection {
        Protection::Read => winapi::PAGE_READONLY,
        Protection::ReadWrite => winapi::PAGE_READWRITE,
        Protection::ReadExecute => winapi::PAGE_EXECUTE_READ,
        Protection::None => winapi::PAGE_NOACCESS,
        _ => winapi::PAGE_EXECUTE_READWRITE,
    }
}

fn convert_from_native(protection: winapi::DWORD) -> Protection {
    // Ignore miscellaneous flags (such as 'PAGE_NOCACHE')
    match (protection & 0xFF) {
        winapi::PAGE_EXECUTE => Protection::Execute,
        winapi::PAGE_EXECUTE_READ => Protection::ReadExecute,
        winapi::PAGE_EXECUTE_READWRITE => Protection::ReadWriteExecute,
        winapi::PAGE_EXECUTE_WRITECOPY => Protection::ReadWriteExecute,
        winapi::PAGE_NOACCESS => Protection::None,
        winapi::PAGE_READONLY => Protection::Read,
        winapi::PAGE_READWRITE => Protection::ReadWrite,
        winapi::PAGE_WRITECOPY => Protection::ReadWrite,
        _ => unreachable!(),
    }
}

pub fn page_size() -> usize {
    use winapi::{GetSystemInfo, SYSTEM_INFO};

    lazy_static! {
        static ref PAGESIZE: usize = unsafe {
            let mut info: SYSTEM_INFO = std::mem::zeroed();
            GetSystemInfo(&mut info);
            return info.dwPageSize as usize;
        };
    }

    return *PAGESIZE;
}

pub fn get_region(base: *const u8) -> Result<Region> {
    extern crate winapi;

    let mut info: winapi::MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    let result = unsafe {
        winapi::VirtualQuery(base,
                             &mut info,
                             std::mem::size_of::<winapi::MEMORY_BASIC_INFORMATION>())
    };

    match result {
        winapi::ERROR_SUCCESS => {
            if info.State == winapi::MEM_FREE {
                return Err(Error::Freed);
            }

            Ok(Region {
                base: info.BaseAddress,
                guarded: (info.Protect & winapi::PAGE_GUARD) != 0,
                protection: convert_from_native(info.Protect),
                shared: !(info.Type & winapi::MEM_PRIVATE),
                size: info.RegionSize,
            })
        }
        _ => Err(Error::VirtualQuery(::errno::Errno(result))),
    }
}

pub fn set_prot(base: *const u8, size: usize, protection: Protection) -> Result<()> {
    let mut prev_flags = 0;
    let result = unsafe {
        winapi::VirtualProtect(base as winapi::PVOID,
                               size,
                               convert_to_native(protection),
                               &mut prev_flags);
    };

    match result {
        winapi::ERROR_SUCCESS => Ok(()),
        _ => Err(Error::VirtualProtect(::errno::Errno(result))),
    }
}
