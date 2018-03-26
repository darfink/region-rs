use Protection;
use error::*;

use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};

fn prot_to_native(protection: Protection) -> ::libc::c_int {
  let mut result = PROT_NONE;
  let prots = [
    (Protection::Read, PROT_READ),
    (Protection::Write, PROT_WRITE),
    (Protection::Execute, PROT_EXEC),
  ];

  for &(prot, unix_flag) in &prots {
    if protection.contains(prot) {
      result |= unix_flag;
    }
  }

  result
}

pub fn page_size() -> usize {
  unsafe { ::libc::sysconf(::libc::_SC_PAGESIZE) as usize }
}

pub fn set_protection(base: *const u8, size: usize, protection: Protection) -> Result<()> {
  let result = unsafe {
    ::libc::mprotect(
      base as *mut ::libc::c_void,
      size,
      prot_to_native(protection),
    )
  };

  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(::errno::errno()).into()),
  }
}

pub fn lock(base: *const u8, size: usize) -> Result<()> {
  let result = unsafe { ::libc::mlock(base as *const ::libc::c_void, size) };
  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(::errno::errno()).into()),
  }
}

pub fn unlock(base: *const u8, size: usize) -> Result<()> {
  let result = unsafe { ::libc::munlock(base as *const ::libc::c_void, size) };
  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(::errno::errno()).into()),
  }
}
