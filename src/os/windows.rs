extern crate winapi;

use std::io;
use {Error, Protection, Region, Result};

impl Protection {
  fn from_native(protection: winapi::shared::minwindef::DWORD) -> Self {
    // Ignore unsupported flags (TODO: Preserve this information?)
    let ignored = winapi::um::winnt::PAGE_GUARD
      | winapi::um::winnt::PAGE_NOCACHE
      | winapi::um::winnt::PAGE_WRITECOMBINE;

    match protection & !ignored {
      winapi::um::winnt::PAGE_EXECUTE => Protection::EXECUTE,
      winapi::um::winnt::PAGE_EXECUTE_READ => Protection::READ_EXECUTE,
      winapi::um::winnt::PAGE_EXECUTE_READWRITE => Protection::READ_WRITE_EXECUTE,
      winapi::um::winnt::PAGE_EXECUTE_WRITECOPY => Protection::READ_WRITE_EXECUTE,
      winapi::um::winnt::PAGE_NOACCESS => Protection::NONE,
      winapi::um::winnt::PAGE_READONLY => Protection::READ,
      winapi::um::winnt::PAGE_READWRITE => Protection::READ_WRITE,
      winapi::um::winnt::PAGE_WRITECOPY => Protection::READ_WRITE,
      _ => unreachable!("Protection: 0x{:X}", protection),
    }
  }

  fn to_native(self) -> winapi::shared::minwindef::DWORD {
    match self {
      Protection::NONE => winapi::um::winnt::PAGE_NOACCESS,
      Protection::READ => winapi::um::winnt::PAGE_READONLY,
      Protection::EXECUTE => winapi::um::winnt::PAGE_EXECUTE,
      Protection::READ_EXECUTE => winapi::um::winnt::PAGE_EXECUTE_READ,
      Protection::READ_WRITE => winapi::um::winnt::PAGE_READWRITE,
      Protection::READ_WRITE_EXECUTE => winapi::um::winnt::PAGE_EXECUTE_READWRITE,
      Protection::WRITE_EXECUTE => winapi::um::winnt::PAGE_EXECUTE_READWRITE,
      _ => unreachable!("Protection: {:?}", self),
    }
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
      winapi::um::winnt::MEM_FREE => Err(Error::FreeMemory)?,
      winapi::um::winnt::MEM_RESERVE => (Protection::NONE, false),
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
