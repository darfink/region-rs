use crate::{Error, Protection, Region, Result};
use mach::vm_prot::*;
use std::iter;
use take_until::TakeUntilExt;

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  // The possible memory share modes
  const SHARE_MODES: [u8; 3] = [
    mach::vm_region::SM_SHARED,
    mach::vm_region::SM_TRUESHARED,
    mach::vm_region::SM_SHARED_ALIASED,
  ];

  // The address is aligned to the enclosing region
  let mut region_base = origin as mach::vm_types::mach_vm_address_t;
  let mut region_size: mach::vm_types::mach_vm_size_t = 0;
  let mut info: mach::vm_region::vm_region_extended_info = unsafe { std::mem::zeroed() };

  let upper_bound = (origin as usize).saturating_add(size);
  let iterator = iter::from_fn(move || {
    // Check if the search area is passed
    if region_base as usize >= upper_bound {
      return None;
    }

    let result = unsafe {
      // This returns the closest region that is at, or after, the address.
      mach::vm::mach_vm_region(
        mach::traps::mach_task_self(),
        &mut region_base,
        &mut region_size,
        mach::vm_region::VM_REGION_EXTENDED_INFO,
        (&mut info as *mut _) as mach::vm_region::vm_region_info_t,
        &mut mach::vm_region::vm_region_extended_info::count(),
        &mut 0,
      )
    };

    match result {
      // The end of the process' address space has been reached
      mach::kern_return::KERN_INVALID_ADDRESS => None,
      mach::kern_return::KERN_SUCCESS => {
        // The returned region may have a different address than the request
        if region_base as usize >= upper_bound {
          return None;
        }

        let region = Region {
          base: region_base as *const _,
          guarded: (info.user_tag == mach::vm_statistics::VM_MEMORY_GUARD),
          protection: Protection::from_native(info.protection),
          shared: SHARE_MODES.contains(&info.share_mode),
          size: region_size as usize,
        };

        region_base = region_base.saturating_add(region_size);
        Some(Ok(region))
      }
      _ => Some(Err(Error::MachCall(result))),
    }
  })
  .take_until(|res| res.is_err())
  .fuse();
  Ok(iterator)
}

impl Protection {
  fn from_native(protection: vm_prot_t) -> Self {
    const MAPPINGS: &[(vm_prot_t, Protection)] = &[
      (VM_PROT_READ, Protection::READ),
      (VM_PROT_WRITE, Protection::WRITE),
      (VM_PROT_EXECUTE, Protection::EXECUTE),
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
    let rw = VM_PROT_READ | VM_PROT_WRITE;
    let rwx = rw | VM_PROT_EXECUTE;

    assert_eq!(Protection::from_native(0), Protection::NONE);
    assert_eq!(Protection::from_native(VM_PROT_READ), Protection::READ);
    assert_eq!(Protection::from_native(rw), Protection::READ_WRITE);
    assert_eq!(Protection::from_native(rwx), Protection::READ_WRITE_EXECUTE);
  }
}
