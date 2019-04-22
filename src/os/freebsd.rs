use libc::{c_int, c_void, getpid, pid_t};
use {Error, Protection, Region, Result};

pub fn get_region(address: *const u8) -> Result<Region> {
  unsafe {
    let mut vm_cnt = 0;
    let vm = kinfo_getvmmap(getpid(), &mut vm_cnt);
    if vm.is_null() {
      return Err(Error::NullAddress);
    }

    for i in 0..vm_cnt {
      // Since the struct size is given in the struct, we can use it to be future-proof
      // (we won't need to update the definition here when new fields are added)
      let entry = &*((vm as *const c_void).offset(i as isize * (*vm).kve_structsize as isize)
        as *const kinfo_vmentry);
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
}

impl Protection {
  fn from_native(protection: c_int) -> Self {
    let mut result = Protection::None;

    if (protection & KVME_PROT_READ) == KVME_PROT_READ {
      result |= Protection::Read;
    }

    if (protection & KVME_PROT_WRITE) == KVME_PROT_WRITE {
      result |= Protection::Write;
    }

    if (protection & KVME_PROT_EXEC) == KVME_PROT_EXEC {
      result |= Protection::Execute;
    }

    result
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
