use crate::{Error, Protection, Region, Result};
use libc::{
  c_int, c_void, free, getpid, kinfo_getvmmap, kinfo_vmentry, KVME_PROT_EXEC, KVME_PROT_READ,
  KVME_PROT_WRITE, KVME_TYPE_DEFAULT,
};
use std::io;

pub struct QueryIter {
  vmmap: *mut kinfo_vmentry,
  vmmap_len: usize,
  vmmap_index: usize,
  upper_bound: usize,
}

impl QueryIter {
  pub fn new(origin: *const (), size: usize) -> Result<QueryIter> {
    let mut vmmap_len = 0;
    let vmmap = unsafe { kinfo_getvmmap(getpid(), &mut vmmap_len) };

    if vmmap.is_null() {
      return Err(Error::SystemCall(io::Error::last_os_error()));
    }

    Ok(QueryIter {
      vmmap,
      vmmap_len: vmmap_len as usize,
      vmmap_index: 0,
      upper_bound: (origin as usize).saturating_add(size),
    })
  }

  pub fn upper_bound(&self) -> usize {
    self.upper_bound
  }
}

impl Iterator for QueryIter {
  type Item = Result<Region>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.vmmap_index >= self.vmmap_len {
      return None;
    }

    // Since the struct size is given in the struct, it can be used future-proof
    // (the definition is not required to be updated when new fields are added).
    let offset = unsafe { self.vmmap_index * (*self.vmmap).kve_structsize as usize };
    let entry = unsafe { &*((self.vmmap as *const c_void).add(offset) as *const kinfo_vmentry) };

    self.vmmap_index += 1;
    Some(Ok(Region {
      base: entry.kve_start as *const _,
      protection: Protection::from_native(entry.kve_protection),
      shared: entry.kve_type == KVME_TYPE_DEFAULT,
      size: (entry.kve_end - entry.kve_start) as _,
      ..Default::default()
    }))
  }
}

impl Drop for QueryIter {
  fn drop(&mut self) {
    unsafe { free(self.vmmap as *mut c_void) }
  }
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
