use libc::{c_int, c_void, free, getpid, pid_t};
use std::io;
use {Error, Protection, Region, Result};

pub fn get_region(address: *const u8) -> Result<Region> {
  let mut vm_cnt = 0;
  let vm = unsafe { kinfo_getvmmap(getpid(), &mut vm_cnt) };

  if vm.is_null() {
    return Err(Error::SystemCall(io::Error::last_os_error()));
  }

  // The caller is expected to free the VM entry
  let _guard = ScopeGuard::new(|| unsafe { free(vm as *mut c_void) });

  for index in 0..vm_cnt {
    // Since the struct size is given in the struct, it can be used future-proof
    // (the definition is not required to be updated when new fields are added).
    let offset = unsafe { index as isize * (*vm).kve_structsize as isize };
    let entry = unsafe { &*((vm as *const c_void).offset(offset) as *const kinfo_vmentry) };

    if address >= entry.kve_start as *const _ && address < entry.kve_end as *const _ {
      return Ok(Region {
        base: entry.kve_start as *const _,
        size: (entry.kve_end - entry.kve_start) as _,
        guarded: false,
        protection: Protection::from_native(entry.kve_protection),
        shared: entry.kve_type == KVME_TYPE_DEFAULT,
      });
    }
  }

  Err(Error::FreeMemory)
}

impl Protection {
  fn from_native(protection: c_int) -> Self {
    let mut result = Protection::NONE;

    if (protection & KVME_PROT_READ) == KVME_PROT_READ {
      result |= Protection::READ;
    }

    if (protection & KVME_PROT_WRITE) == KVME_PROT_WRITE {
      result |= Protection::WRITE;
    }

    if (protection & KVME_PROT_EXEC) == KVME_PROT_EXEC {
      result |= Protection::EXECUTE;
    }

    result
  }
}

struct ScopeGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> ScopeGuard<F> {
  pub fn new(closure: F) -> Self {
    ScopeGuard(Some(closure))
  }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
  fn drop(&mut self) {
    self.0.take().unwrap()()
  }
}

#[repr(C)]
struct kinfo_vmentry {
  kve_structsize: c_int,
  kve_type: c_int,
  kve_start: u64,
  kve_end: u64,
  kve_offset: u64,
  kve_vn_fileid: u64,
  kve_vn_fsid_freebsd11: u32,
  kve_flags: c_int,
  kve_resident: c_int,
  kve_private_resident: c_int,
  kve_protection: c_int,
}

const KVME_TYPE_DEFAULT: c_int = 1;
const KVME_PROT_READ: c_int = 1;
const KVME_PROT_WRITE: c_int = 2;
const KVME_PROT_EXEC: c_int = 4;

#[link(name = "util")]
extern "C" {
  fn kinfo_getvmmap(pid: pid_t, cntp: *mut c_int) -> *mut kinfo_vmentry;
}
