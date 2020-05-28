extern crate mach;

use self::mach::vm_prot::*;
use {Error, Protection, Region, Result};

impl Protection {
  fn from_native(protection: vm_prot_t) -> Self {
    let mut result = Protection::NONE;

    if (protection & VM_PROT_READ) == VM_PROT_READ {
      result |= Protection::READ;
    }

    if (protection & VM_PROT_WRITE) == VM_PROT_WRITE {
      result |= Protection::WRITE;
    }

    if (protection & VM_PROT_EXECUTE) == VM_PROT_EXECUTE {
      result |= Protection::EXECUTE;
    }

    result
  }
}

pub fn get_region(base: *const u8) -> Result<Region> {
  extern crate mach;

  // Defines the different share modes available
  const SHARE_MODES: [u8; 3] = [
    mach::vm_region::SM_SHARED,
    mach::vm_region::SM_TRUESHARED,
    mach::vm_region::SM_SHARED_ALIASED,
  ];

  // The address is aligned to the enclosing region
  let mut region_base = base as mach::vm_types::mach_vm_address_t;
  let mut region_size: mach::vm_types::mach_vm_size_t = 0;
  let mut info: mach::vm_region::vm_region_extended_info = unsafe { ::std::mem::zeroed() };

  let result = unsafe {
    // This information is of no interest
    let mut object_name: mach::port::mach_port_t = 0;

    // Query the OS about the memory region
    mach::vm::mach_vm_region(
      mach::traps::mach_task_self(),
      &mut region_base,
      &mut region_size,
      mach::vm_region::VM_REGION_EXTENDED_INFO,
      (&mut info as *mut _) as mach::vm_region::vm_region_info_t,
      &mut mach::vm_region::vm_region_extended_info::count(),
      &mut object_name,
    )
  };

  match result {
    mach::kern_return::KERN_SUCCESS => {
      // `mach_vm_region` begins searching at the specified address, so if
      // there is not a region allocated, it will return the closest one
      // instead. In that case, the memory is not committed.
      if region_base > base as mach::vm_types::mach_vm_address_t {
        Err(Error::FreeMemory)
      } else {
        Ok(Region {
          base: region_base as *const _,
          guarded: (info.user_tag == mach::vm_statistics::VM_MEMORY_GUARD),
          protection: Protection::from_native(info.protection),
          shared: SHARE_MODES.contains(&info.share_mode),
          size: region_size as usize,
        })
      }
    }
    _ => Err(Error::MachRegion(result)),
  }
}
