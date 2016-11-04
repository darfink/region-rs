extern crate winapi;
extern crate kernel32;

use Error;
use Protection;
use Region;

fn convert_to_native(protection: Protection::Flag) -> winapi::DWORD {
    match protection {
        Protection::Read => winapi::PAGE_READONLY,
        Protection::ReadWrite => winapi::PAGE_READWRITE,
        Protection::ReadExecute => winapi::PAGE_EXECUTE_READ,
        Protection::None => winapi::PAGE_NOACCESS,
        _ => winapi::PAGE_EXECUTE_READWRITE,
    }
}

fn convert_from_native(protection: winapi::DWORD) -> Protection::Flag {
    // Ignore miscellaneous flags (such as 'PAGE_NOCACHE')
    match protection & 0xFF {
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
    use self::kernel32::GetSystemInfo;
    use self::winapi::SYSTEM_INFO;

    lazy_static! {
        static ref PAGESIZE: usize = unsafe {
            let mut info: SYSTEM_INFO = ::std::mem::zeroed();
            GetSystemInfo(&mut info);
            return info.dwPageSize as usize;
        };
    }

    return *PAGESIZE;
}

pub fn get_region(base: *const u8) -> Result<Region, Error> {
    use self::kernel32::VirtualQuery;

    let mut info: winapi::MEMORY_BASIC_INFORMATION = unsafe { ::std::mem::zeroed() };
    let bytes = unsafe {
        VirtualQuery(base as winapi::PVOID,
                     &mut info,
                     ::std::mem::size_of::<winapi::MEMORY_BASIC_INFORMATION>() as winapi::SIZE_T)
    };

    if bytes > 0 {
        if info.State == winapi::MEM_FREE {
            return Err(Error::Freed);
        }

        Ok(Region {
            base: info.BaseAddress as *mut u8,
            guarded: (info.Protect & winapi::PAGE_GUARD) != 0,
            protection: convert_from_native(info.Protect),
            shared: (info.Type & winapi::MEM_PRIVATE) == 0,
            size: info.RegionSize as usize,
        })
    } else {
        Err(Error::VirtualQuery(::errno::errno()))
    }
}

pub fn set_prot(base: *const u8, size: usize, protection: Protection::Flag) -> Result<(), Error> {
    use self::kernel32::VirtualProtect;

    let mut prev_flags = 0;
    let result = unsafe {
        VirtualProtect(base as winapi::PVOID,
                               size as winapi::SIZE_T,
                               convert_to_native(protection),
                               &mut prev_flags)
    };

    if result == winapi::FALSE {
        Err(Error::VirtualProtect(::errno::errno()))
    } else {
        Ok(())
    }
}
