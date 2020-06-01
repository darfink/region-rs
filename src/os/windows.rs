use crate::{Error, Protection, Region, Result};
use std::cmp::{max, min};
use std::mem::{size_of, MaybeUninit};
use std::sync::Once;
use std::{io, iter};
use take_until::TakeUntilExt;
use winapi::um::memoryapi::{VirtualLock, VirtualProtect, VirtualQuery, VirtualUnlock};
use winapi::um::sysinfoapi::{GetNativeSystemInfo, SYSTEM_INFO};
use winapi::um::winnt::MEMORY_BASIC_INFORMATION;

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  let system = system_info();

  let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
  let mut region_base = max(origin as usize, system.lpMinimumApplicationAddress as usize);

  let upper_bound = min(
    (origin as usize).saturating_add(size),
    system.lpMaximumApplicationAddress as usize,
  );

  let iterator = iter::from_fn(move || {
    while region_base < upper_bound {
      let bytes = unsafe {
        VirtualQuery(
          region_base as winapi::um::winnt::PVOID,
          &mut info,
          size_of::<MEMORY_BASIC_INFORMATION>() as winapi::shared::basetsd::SIZE_T,
        )
      };

      if bytes == 0 {
        return Some(Err(Error::SystemCall(io::Error::last_os_error())));
      }

      region_base = (info.BaseAddress as usize).saturating_add(info.RegionSize);

      // Only mapped memory regions are of interest
      if info.State == winapi::um::winnt::MEM_COMMIT {
        return Some(Ok(Region {
          base: info.BaseAddress as *const _,
          shared: (info.Type & winapi::um::winnt::MEM_PRIVATE) == 0,
          size: info.RegionSize as usize,
          protection: Protection::from_native(info.Protect),
          guarded: (info.Protect & winapi::um::winnt::PAGE_GUARD) != 0,
        }));
      }
    }

    None
  })
  .take_until(|res| res.is_err())
  .fuse();
  Ok(iterator)
}

pub fn page_size() -> usize {
  system_info().dwPageSize as usize
}

pub unsafe fn protect<T>(base: *const T, size: usize, protection: Protection) -> Result<()> {
  let mut prev_flags = 0;
  let result = VirtualProtect(
    base as winapi::um::winnt::PVOID,
    size as winapi::shared::basetsd::SIZE_T,
    protection.to_native(),
    &mut prev_flags,
  );

  if result == winapi::shared::minwindef::FALSE {
    Err(Error::SystemCall(io::Error::last_os_error()))
  } else {
    Ok(())
  }
}

pub fn lock<T>(base: *const T, size: usize) -> Result<()> {
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

pub fn unlock<T>(base: *const T, size: usize) -> Result<()> {
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

fn system_info() -> &'static SYSTEM_INFO {
  static INIT: Once = Once::new();
  static mut INFO: MaybeUninit<SYSTEM_INFO> = MaybeUninit::uninit();

  unsafe {
    INIT.call_once(|| GetNativeSystemInfo(INFO.as_mut_ptr()));
    &*INFO.as_ptr()
  }
}

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
