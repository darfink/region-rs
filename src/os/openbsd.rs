use crate::{Error, Protection, Region, Result};
use libc::{c_int, c_ulong, getpid, sysctl, CTL_KERN, KERN_PROC_VMMAP};
use std::io;
use std::iter;
use take_until::TakeUntilExt;

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  let upper_bound = (origin as usize).saturating_add(size);
  let mib: [c_int; 3] = [CTL_KERN, KERN_PROC_VMMAP, unsafe { getpid() }];
  let mut len = std::mem::size_of::<kinfo_vmentry>();
  let mut entry: kinfo_vmentry = unsafe { std::mem::zeroed() };
  let mut old_end = 0;

  let iterator = iter::from_fn(move || {
    let result = unsafe {
      sysctl(
        mib.as_ptr(),
        mib.len() as u32,
        &mut entry as *mut _ as *mut _,
        &mut len,
        std::ptr::null_mut(),
        0,
      )
    };

    if result == -1 {
      return Some(Err(Error::SystemCall(io::Error::last_os_error())));
    }

    if entry.kve_end == old_end {
      return None;
    }

    let region = Region {
      base: entry.kve_start as *const _,
      protection: Protection::from_native(entry.kve_protection),
      shared: (entry.kve_etype & KVE_ET_COPYONWRITE) == 0,
      size: (entry.kve_end - entry.kve_start) as _,
      ..Default::default()
    };

    old_end = entry.kve_end;
    entry.kve_start += 1;

    Some(Ok(region))
  })
  .skip_while(move |res| matches!(res, Ok(region) if region.as_range().end <= origin as usize))
  .take_while(move |res| !matches!(res, Ok(region) if region.as_range().start >= upper_bound))
  .take_until(|res| res.is_err())
  .fuse();
  Ok(iterator)
}

impl Protection {
  fn from_native(protection: c_int) -> Self {
    const MAPPINGS: &[(c_int, Protection)] = &[
      (KVE_PROT_READ, Protection::READ),
      (KVE_PROT_WRITE, Protection::WRITE),
      (KVE_PROT_EXEC, Protection::EXECUTE),
    ];

    MAPPINGS
      .iter()
      .filter(|(flag, _)| protection & *flag == *flag)
      .fold(Protection::NONE, |acc, (_, prot)| acc | *prot)
  }
}

#[repr(C)]
struct kinfo_vmentry {
  kve_start: c_ulong,
  kve_end: c_ulong,
  kve_guard: c_ulong,
  kve_fspace: c_ulong,
  kve_fspace_augment: c_ulong,
  kve_offset: u64,
  kve_wired_count: c_int,
  kve_etype: c_int,
  kve_protection: c_int,
  kve_max_protection: c_int,
  kve_advice: c_int,
  kve_inheritance: c_int,
  kve_flags: u8,
}

const KVE_PROT_READ: c_int = 1;
const KVE_PROT_WRITE: c_int = 2;
const KVE_PROT_EXEC: c_int = 4;
const KVE_ET_COPYONWRITE: c_int = 4;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn protection_flags_are_mapped_from_native() {
    let rw = KVE_PROT_READ | KVE_PROT_WRITE;
    let rwx = rw | KVE_PROT_EXEC;

    assert_eq!(Protection::from_native(0), Protection::NONE);
    assert_eq!(Protection::from_native(KVE_PROT_READ), Protection::READ);
    assert_eq!(Protection::from_native(rw), Protection::READ_WRITE);
    assert_eq!(Protection::from_native(rwx), Protection::READ_WRITE_EXECUTE);
  }
}
