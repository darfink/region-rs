extern crate winapi;

use Protection;

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

impl From<Protection> for winapi::DWORD {
    fn from(protection: Protection) -> Self {
        match protection {
            Protection::Read        => winapi::PAGE_READONLY,
            Protection::ReadWrite   => winapi::PAGE_READWRITE,
            Protection::ReadExecute => winapi::PAGE_EXECUTE_READ,
            Protection::None        => winapi::PAGE_NOACCESS,
            _                       => winapi::PAGE_EXECUTE_READWRITE,
        }
    }
}

impl From<winapi::DWORD> for Protection {
    fn from(protection: winapi::DWORD) -> Self {
        // Ignore miscellaneous flags (such as 'PAGE_NOCACHE')
        match (protection & 0xFF) {
            winapi::PAGE_EXECUTE           => Protection::Execute,
            winapi::PAGE_EXECUTE_READ      => Protection::ReadExecute,
            winapi::PAGE_EXECUTE_READWRITE => Protection::ReadWriteExecute,
            winapi::PAGE_EXECUTE_WRITECOPY => Protection::ReadWriteExecute,
            winapi::PAGE_NOACCESS          => Protection::None,
            winapi::PAGE_READONLY          => Protection::Read,
            winapi::PAGE_READWRITE         => Protection::ReadWrite,
            winapi::PAGE_WRITECOPY         => Protection::ReadWrite,
            _                              => unreachable!(),
        }
    }
}
