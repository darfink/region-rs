use crate::{Error, Protection, Region, Result};
use libc::{c_int, c_void, free, getpid, pid_t};
use std::io;

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  let mut vm_cnt = 0;
  let vm = unsafe { kinfo_getvmmap(getpid(), &mut vm_cnt) };

  if vm.is_null() {
    return Err(Error::SystemCall(io::Error::last_os_error()));
  }

  // The caller is expected to free the VM entry
  let guard = ScopeGuard::new(move || unsafe { free(vm as *mut c_void) });
  let upper_bound = (origin as usize).saturating_add(size);

  let iterator = (0..vm_cnt)
    .map(move |index| {
      // This must be moved into the closure to ensure its lifetime
      let _guard = &guard;

      // Since the struct size is given in the struct, it can be used future-proof
      // (the definition is not required to be updated when new fields are added).
      let offset = unsafe { index as isize * (*vm).kve_structsize as isize };
      let entry = unsafe { &*((vm as *const c_void).offset(offset) as *const kinfo_vmentry) };

      Region {
        base: entry.kve_start as *const _,
        size: (entry.kve_end - entry.kve_start) as _,
        guarded: false,
        protection: Protection::from_native(entry.kve_protection),
        shared: entry.kve_type == KVME_TYPE_DEFAULT,
      }
    })
    .skip_while(move |region| region.as_range().end <= origin as usize)
    .take_while(move |region| region.as_range().start < upper_bound)
    .map(Ok);
  Ok(iterator)
}

impl Protection {
  fn from_native(protection: c_int) -> Self {
    const MAPPINGS: &[(c_int, Protection)] = &[
      (KVME_PROT_READ, Protection::READ),
      (KVME_PROT_WRITE, Protection::WRITE),
      (KVME_PROT_EXEC, Protection::EXECUTE),
    ];

    MAPPINGS
      .iter()
      .filter(|(flag, _)| protection & *flag == *flag)
      .fold(Protection::NONE, |acc, (_, prot)| acc | *prot)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn protection_flags_are_mapped_from_native() {
    let rw = KVME_PROT_READ | KVME_PROT_WRITE;
    let rwx = rw | KVME_PROT_EXEC;

    assert_eq!(Protection::from_native(0), Protection::NONE);
    assert_eq!(Protection::from_native(KVME_PROT_READ), Protection::READ);
    assert_eq!(Protection::from_native(rw), Protection::READ_WRITE);
    assert_eq!(Protection::from_native(rwx), Protection::READ_WRITE_EXECUTE);
  }
}
