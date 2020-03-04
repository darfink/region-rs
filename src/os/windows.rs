extern crate winapi;

use std::io;
use {Error, Protection, Region, Result};
use std::ops::Generator;

impl Protection {
  fn from_native(protection: winapi::shared::minwindef::DWORD) -> Self {
    // Ignore unsupported flags (TODO: Preserve this information?)
    let ignored = winapi::um::winnt::PAGE_GUARD
      | winapi::um::winnt::PAGE_NOCACHE
      | winapi::um::winnt::PAGE_WRITECOMBINE;

    match protection & !ignored {
      winapi::um::winnt::PAGE_EXECUTE => Protection::Execute,
      winapi::um::winnt::PAGE_EXECUTE_READ => Protection::ReadExecute,
      winapi::um::winnt::PAGE_EXECUTE_READWRITE => Protection::ReadWriteExecute,
      winapi::um::winnt::PAGE_EXECUTE_WRITECOPY => Protection::ReadWriteExecute,
      winapi::um::winnt::PAGE_NOACCESS => Protection::None,
      winapi::um::winnt::PAGE_READONLY => Protection::Read,
      winapi::um::winnt::PAGE_READWRITE => Protection::ReadWrite,
      winapi::um::winnt::PAGE_WRITECOPY => Protection::ReadWrite,
      _ => unreachable!("Protection: 0x{:X}", protection),
    }
  }

  fn to_native(self) -> winapi::shared::minwindef::DWORD {
    match self {
      Protection::None => winapi::um::winnt::PAGE_NOACCESS,
      Protection::Read => winapi::um::winnt::PAGE_READONLY,
      Protection::Execute => winapi::um::winnt::PAGE_EXECUTE,
      Protection::ReadExecute => winapi::um::winnt::PAGE_EXECUTE_READ,
      Protection::ReadWrite => winapi::um::winnt::PAGE_READWRITE,
      Protection::ReadWriteExecute => winapi::um::winnt::PAGE_EXECUTE_READWRITE,
      Protection::WriteExecute => winapi::um::winnt::PAGE_EXECUTE_READWRITE,
      _ => unreachable!("Protection: {:?}", self),
    }
  }
}

use self::winapi::um::sysinfoapi::SYSTEM_INFO;

pub fn get_system_info() -> SYSTEM_INFO {
  use self::winapi::um::sysinfoapi::GetSystemInfo;
  unsafe {
    let mut info: SYSTEM_INFO = ::std::mem::zeroed();
    GetSystemInfo(&mut info);
    info
  }
}

pub fn page_size() -> usize {
  get_system_info().dwPageSize as usize
}

use self::winapi::um::winnt::MEMORY_BASIC_INFORMATION;

fn mbi_to_region(info: MEMORY_BASIC_INFORMATION) -> Result<Region> {
  let (protection, guarded) = match info.State {
    winapi::um::winnt::MEM_FREE => Err(Error::FreeMemory)?,
    winapi::um::winnt::MEM_RESERVE => (Protection::None, false),
    winapi::um::winnt::MEM_COMMIT => (
      Protection::from_native(info.Protect),
      (info.Protect & winapi::um::winnt::PAGE_GUARD) != 0,
    ),
    _ => unreachable!("State: {}", info.State),
  };

  Ok(Region {
    base: info.BaseAddress as *const _,
    shared: (info.Type & winapi::um::winnt::MEM_PRIVATE) == 0,
    size: info.RegionSize as usize,
    protection,
    guarded,
  })
}

pub fn enumerate_regions(
  handle: self::winapi::um::winnt::HANDLE,
) -> Result<impl Generator<Yield = Result<Region>, Return = ()>> {
  use self::winapi::um::memoryapi::VirtualQueryEx;
  let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { ::std::mem::zeroed() };

  let sys_info = get_system_info();
  let mut addr = sys_info.lpMinimumApplicationAddress as usize;

  Ok(move || {
    while addr < sys_info.lpMaximumApplicationAddress as usize {
      let bytes = unsafe {
        VirtualQueryEx(
          handle, 
          addr as *mut libc::c_void, 
          &mut mbi, 
          ::std::mem::size_of::<MEMORY_BASIC_INFORMATION>() as winapi::shared::basetsd::SIZE_T
        )
      };
      if bytes > 0 {
        let result = mbi_to_region(mbi);
        if let Err(Error::FreeMemory) = result {
          // Not a error for enumeration
        }else {
          yield mbi_to_region(mbi);
        }
      } else {
        yield Err(Error::SystemCall(io::Error::last_os_error()));
        // Cant continue after this
        return;
      }
      addr = mbi.BaseAddress as usize + mbi.RegionSize as usize;
    }
  })
}

pub fn get_region(base: *const u8) -> Result<Region> {
  use self::winapi::um::memoryapi::VirtualQuery;

  let mut info: MEMORY_BASIC_INFORMATION = unsafe { ::std::mem::zeroed() };
  let bytes = unsafe {
    VirtualQuery(
      base as winapi::um::winnt::PVOID,
      &mut info,
      ::std::mem::size_of::<MEMORY_BASIC_INFORMATION>() as winapi::shared::basetsd::SIZE_T,
    )
  };

  if bytes > 0 {
    mbi_to_region(info)
  } else {
    Err(Error::SystemCall(io::Error::last_os_error()))
  }
}

pub fn set_protection(base: *const u8, size: usize, protection: Protection) -> Result<()> {
  use self::winapi::um::memoryapi::VirtualProtect;

  let mut prev_flags = 0;
  let result = unsafe {
    VirtualProtect(
      base as winapi::um::winnt::PVOID,
      size as winapi::shared::basetsd::SIZE_T,
      protection.to_native(),
      &mut prev_flags,
    )
  };

  if result == winapi::shared::minwindef::FALSE {
    Err(Error::SystemCall(io::Error::last_os_error()))
  } else {
    Ok(())
  }
}

pub fn lock(base: *const u8, size: usize) -> Result<()> {
  use self::winapi::um::memoryapi::VirtualLock;
  let result = unsafe {
    VirtualLock(
      base as winapi::um::winnt::PVOID,
      size as winapi::shared::basetsd::SIZE_T,
    )
  };

  if result == winapi::shared::minwindef::FALSE {
    Err(Error::SystemCall(io::Error::last_os_error()))
  } else {
    Ok(())
  }
}

pub fn unlock(base: *const u8, size: usize) -> Result<()> {
  use self::winapi::um::memoryapi::VirtualUnlock;
  let result = unsafe {
    VirtualUnlock(
      base as winapi::um::winnt::PVOID,
      size as winapi::shared::basetsd::SIZE_T,
    )
  };

  if result == winapi::shared::minwindef::FALSE {
    Err(Error::SystemCall(io::Error::last_os_error()))
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::enumerate_regions;

  #[test]
  fn enumerate_regions_works() {
    use gen_iter::GenIter;
    assert_ne!(GenIter(enumerate_regions(-1isize as *mut libc::c_void).expect("failed to init enumeration")).collect::<Vec<_>>().len(), 0);
  }
}