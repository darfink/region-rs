extern crate winapi;

use Protection;
use Region;
use error::*;

fn prot_to_native(protection: Protection) -> winapi::shared::minwindef::DWORD {
  match protection {
    Protection::Read => winapi::um::winnt::PAGE_READONLY,
    Protection::ReadWrite => winapi::um::winnt::PAGE_READWRITE,
    Protection::ReadExecute => winapi::um::winnt::PAGE_EXECUTE_READ,
    Protection::None => winapi::um::winnt::PAGE_NOACCESS,
    _ => winapi::um::winnt::PAGE_EXECUTE_READWRITE,
  }
}

fn prot_from_native(protection: winapi::shared::minwindef::DWORD) -> Protection {
  // Ignore irrelevant flags
  let ignored = winapi::um::winnt::PAGE_GUARD | winapi::um::winnt::PAGE_NOCACHE
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
    _ => unreachable!("Protection: {}", protection),
  }
}

pub fn page_size() -> usize {
  use self::winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

  unsafe {
    let mut info: SYSTEM_INFO = ::std::mem::zeroed();
    GetSystemInfo(&mut info);

    info.dwPageSize as usize
  }
}

pub fn get_region(base: *const u8) -> Result<Region> {
  use self::winapi::um::memoryapi::VirtualQuery;
  use self::winapi::um::winnt::MEMORY_BASIC_INFORMATION;

  let mut info: MEMORY_BASIC_INFORMATION = unsafe { ::std::mem::zeroed() };
  let bytes = unsafe {
    VirtualQuery(
      base as winapi::um::winnt::PVOID,
      &mut info,
      ::std::mem::size_of::<MEMORY_BASIC_INFORMATION>() as winapi::shared::basetsd::SIZE_T,
    )
  };

  if bytes > 0 {
    let (protection, guarded) = match info.State {
      winapi::um::winnt::MEM_FREE => Err(Error::Free)?,
      winapi::um::winnt::MEM_RESERVE => (Protection::None, false),
      winapi::um::winnt::MEM_COMMIT => (
        prot_from_native(info.Protect),
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
  } else {
    Err(Error::SystemCall(::errno::errno()).into())
  }
}

pub fn set_protection(base: *const u8, size: usize, protection: Protection) -> Result<()> {
  use self::winapi::um::memoryapi::VirtualProtect;

  let mut prev_flags = 0;
  let result = unsafe {
    VirtualProtect(
      base as winapi::um::winnt::PVOID,
      size as winapi::shared::basetsd::SIZE_T,
      prot_to_native(protection),
      &mut prev_flags,
    )
  };

  if result == winapi::shared::minwindef::FALSE {
    Err(Error::SystemCall(::errno::errno()).into())
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
    Err(Error::SystemCall(::errno::errno()).into())
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
    Err(Error::SystemCall(::errno::errno()).into())
  } else {
    Ok(())
  }
}
