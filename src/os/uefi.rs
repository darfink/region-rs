use crate::{Error, Protection, Region, Result};
use r_efi::efi;
use std::{io, os::uefi::env, ptr::addr_of};

pub struct QueryIter {
  upper_bound: usize,
}

impl QueryIter {
  pub fn new(origin: *const (), size: usize) -> Result<Self> {
    Ok(Self {
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
    None
  }
}

fn get_mem_attrib_proto() -> Result<*mut core::ffi::c_void> {
  let boot_services = env::boot_services().unwrap().as_ptr() as *mut efi::BootServices;

  unsafe {
    let mut guid = r_efi::protocols::memory_attribute::PROTOCOL_GUID;
    let mut proto: *mut core::ffi::c_void = core::ptr::null_mut();
    let r = ((*boot_services).locate_protocol)(&mut guid, core::ptr::null_mut(), &mut proto);

    match r {
      efi::Status::SUCCESS => Ok(proto),
      efi::Status::NOT_FOUND => Err(Error::SystemCall(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not locate EFI_MEMORY_ATTRIBUTE_PROTOCOL",
      ))),
      efi::Status::INVALID_PARAMETER => Err(Error::InvalidParameter("Protocol is NULL")),
      _ => panic!(),
    }
  }
}

fn get_prot(protection: Protection) -> u64 {
  let mut prot = 0;

  if (protection & Protection::WRITE) != Protection::WRITE {
    prot |= efi::MEMORY_RO;
  }

  if (protection & Protection::EXECUTE) != Protection::EXECUTE {
    prot |= efi::MEMORY_XP;
  }

  if (protection & Protection::READ) != Protection::READ {
    prot |= efi::MEMORY_RP;
  }

  prot
}

pub unsafe fn protect(base: *const (), size: usize, protection: Protection) -> Result<()> {
  let proto = get_mem_attrib_proto()? as *mut r_efi::protocols::memory_attribute::Protocol;
  let prot = get_prot(protection);

  let r = ((*proto).set_memory_attributes)(
    proto,
    addr_of!(base) as efi::PhysicalAddress,
    size as efi::PhysicalAddress,
    prot,
  );

  match r {
    efi::Status::SUCCESS => Ok(()),
    efi::Status::INVALID_PARAMETER => {
      Err(Error::InvalidParameter("Length is 0 or invalid protection"))
    }
    efi::Status::UNSUPPORTED => Err(Error::InvalidParameter(
      "System does not support this operation",
    )),
    efi::Status::OUT_OF_RESOURCES => Err(Error::InvalidParameter("Out of system resources")),
    efi::Status::ACCESS_DENIED => Err(Error::InvalidParameter("Cannot modify firmware address")),
    _ => panic!(),
  }
}

pub unsafe fn alloc(base: *const (), size: usize, protection: Protection) -> Result<*const ()> {
  let boot_services = env::boot_services().unwrap().as_ptr() as *mut efi::BootServices;

  let pages = (size + 4095) / 4096;

  let mut addr = addr_of!(base) as efi::PhysicalAddress;

  let alloc_type = if base.is_null() {
    efi::ALLOCATE_ANY_PAGES
  } else {
    efi::ALLOCATE_ADDRESS
  };

  let r = ((*boot_services).allocate_pages)(alloc_type, efi::LOADER_DATA, pages, &mut addr);

  match r {
    efi::Status::SUCCESS => {
      protect(addr as *const (), pages * 4096, protection)?;
      Ok(addr as *const ())
    }
    efi::Status::INVALID_PARAMETER => Err(Error::InvalidParameter(
      "Base address or memory type is not valid",
    )),
    efi::Status::OUT_OF_RESOURCES => Err(Error::InvalidParameter("Out of system resources")),
    efi::Status::NOT_FOUND => Err(Error::InvalidParameter("Could not find a page")),
    _ => panic!(),
  }
}

pub unsafe fn free(base: *const (), size: usize) -> Result<()> {
  let boot_services = env::boot_services().unwrap().as_ptr() as *mut efi::BootServices;

  let pages = (size + 4095) / 4096;

  let r = ((*boot_services).free_pages)(addr_of!(base) as efi::PhysicalAddress, pages);

  match r {
    efi::Status::SUCCESS => Ok(()),
    efi::Status::INVALID_PARAMETER => Err(Error::InvalidParameter(
      "Address it not page aligned or is invalid",
    )),
    efi::Status::NOT_FOUND => Err(Error::InvalidParameter("Could not find the allocation")),
    _ => panic!(),
  }
}

pub fn page_size() -> usize {
  4096
}
