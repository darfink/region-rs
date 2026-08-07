use crate::{Error, Protection, Region, Result};
use core::cmp::{max, min};
use core::ffi::c_void;
use core::mem::{MaybeUninit, size_of};
use core::ptr;
use core::sync::Once;
use windows_sys::Win32::System::Memory::{
  MEM_COMMIT, MEM_PRIVATE, MEM_RELEASE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE,
  PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS,
  PAGE_NOCACHE, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOMBINE, PAGE_WRITECOPY, VirtualAlloc,
  VirtualFree, VirtualLock, VirtualProtect, VirtualQuery, VirtualUnlock,
};
use windows_sys::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};

pub struct QueryIter {
  region_address: usize,
  upper_bound: usize,
}

impl QueryIter {
  pub fn new(origin: *const (), size: usize) -> Result<QueryIter> {
    let system = system_info();

    Ok(QueryIter {
      region_address: max(origin.addr(), system.lpMinimumApplicationAddress as usize),
      upper_bound: min(
        origin.addr().saturating_add(size),
        system.lpMaximumApplicationAddress as usize,
      ),
    })
  }

  pub fn upper_bound(&self) -> usize {
    self.upper_bound
  }
}

impl Iterator for QueryIter {
  type Item = Result<Region>;

  fn next(&mut self) -> Option<Self::Item> {
    let mut info = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();

    while self.region_address < self.upper_bound {
      let bytes = unsafe {
        VirtualQuery(
          ptr::with_exposed_provenance::<c_void>(self.region_address),
          info.as_mut_ptr(),
          size_of::<MEMORY_BASIC_INFORMATION>(),
        )
      };

      if bytes == 0 {
        return Some(Err(Error::last_os_error()));
      }

      let info = unsafe { info.assume_init() };
      self.region_address = (info.BaseAddress as usize).saturating_add(info.RegionSize);

      // Only mapped memory regions are of interest
      if info.State == MEM_RESERVE || info.State == MEM_COMMIT {
        let mut region = Region {
          base: ptr::with_exposed_provenance(info.BaseAddress as usize),
          reserved: info.State != MEM_COMMIT,
          guarded: (info.Protect & PAGE_GUARD) != 0,
          shared: (info.Type & MEM_PRIVATE) == 0,
          size: info.RegionSize,
          ..Default::default()
        };

        if region.is_committed() {
          region.protection = Protection::from_native(info.Protect);
          region.max_protection = region.protection;
        }

        return Some(Ok(region));
      }
    }

    None
  }
}

pub fn page_size() -> usize {
  system_info().dwPageSize as usize
}

pub unsafe fn alloc(base: *const (), size: usize, protection: Protection) -> Result<*const ()> {
  // Reserving without committing is required for large `Protection::NONE`
  // allocations that only need address space.
  let allocation_type = if protection == Protection::NONE {
    MEM_RESERVE
  } else {
    MEM_COMMIT | MEM_RESERVE
  };

  let allocation = unsafe {
    VirtualAlloc(
      base.cast_mut().cast(),
      size,
      allocation_type,
      protection.to_native(),
    )
  };

  if allocation.is_null() {
    return Err(Error::last_os_error());
  }

  Ok(allocation.cast())
}

pub unsafe fn free(base: *const (), _size: usize) -> Result<()> {
  match unsafe { VirtualFree(base.cast_mut().cast(), 0, MEM_RELEASE) } {
    0 => Err(Error::last_os_error()),
    _ => Ok(()),
  }
}

pub unsafe fn protect(base: *const (), size: usize, protection: Protection) -> Result<()> {
  let mut old_protect = 0;
  let result = unsafe {
    VirtualProtect(
      base.cast_mut().cast(),
      size,
      protection.to_native(),
      &mut old_protect,
    )
  };

  if result == 0 {
    Err(Error::last_os_error())
  } else {
    Ok(())
  }
}

pub fn lock(base: *const (), size: usize) -> Result<()> {
  let result = unsafe { VirtualLock(base.cast_mut().cast(), size) };

  if result == 0 {
    Err(Error::last_os_error())
  } else {
    Ok(())
  }
}

pub fn unlock(base: *const (), size: usize) -> Result<()> {
  let result = unsafe { VirtualUnlock(base.cast_mut().cast(), size) };

  if result == 0 {
    Err(Error::last_os_error())
  } else {
    Ok(())
  }
}

// `SYSTEM_INFO` contains two `*mut c_void` pointers, but they are only used as
// numerical values and never dereferenced. Hence, it's safe to share.
struct SystemInfo(SYSTEM_INFO);

unsafe impl Send for SystemInfo {}
unsafe impl Sync for SystemInfo {}

fn system_info() -> &'static SYSTEM_INFO {
  static INIT: Once = Once::new();
  static mut INFO: MaybeUninit<SystemInfo> = MaybeUninit::uninit();

  INIT.call_once(|| {
    let mut info = MaybeUninit::<SYSTEM_INFO>::uninit();
    unsafe {
      GetNativeSystemInfo(info.as_mut_ptr());
      // SAFETY: call_once guarantees single-threaded initialization.
      INFO.write(SystemInfo(info.assume_init()));
    }
  });

  // SAFETY: INFO is initialized exactly once above before any reader reaches here.
  unsafe { &INFO.assume_init_ref().0 }
}

impl Protection {
  fn from_native(protection: u32) -> Self {
    // Ignore unsupported flags (TODO: Preserve this information?)
    let ignored = PAGE_GUARD | PAGE_NOCACHE | PAGE_WRITECOMBINE;

    match protection & !ignored {
      PAGE_EXECUTE => Protection::EXECUTE,
      PAGE_EXECUTE_READ => Protection::READ_EXECUTE,
      PAGE_EXECUTE_READWRITE => Protection::READ_WRITE_EXECUTE,
      PAGE_EXECUTE_WRITECOPY => Protection::READ_WRITE_EXECUTE,
      PAGE_NOACCESS => Protection::NONE,
      PAGE_READONLY => Protection::READ,
      PAGE_READWRITE => Protection::READ_WRITE,
      PAGE_WRITECOPY => Protection::READ_WRITE,
      _ => unreachable!("Protection: 0x{:X}", protection),
    }
  }

  pub(crate) fn to_native(self) -> u32 {
    match self {
      Protection::NONE => PAGE_NOACCESS,
      Protection::READ => PAGE_READONLY,
      Protection::EXECUTE => PAGE_EXECUTE,
      Protection::READ_EXECUTE => PAGE_EXECUTE_READ,
      Protection::READ_WRITE => PAGE_READWRITE,
      Protection::READ_WRITE_EXECUTE => PAGE_EXECUTE_READWRITE,
      Protection::WRITE_EXECUTE => PAGE_EXECUTE_READWRITE,
      _ => unreachable!("Protection: {:?}", self),
    }
  }
}
