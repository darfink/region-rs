use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};
use std::io;
use {Error, Protection, Result};

impl Protection {
  fn to_native(self) -> ::libc::c_int {
    let mut result = PROT_NONE;
    let prots = [
      (Protection::READ, PROT_READ),
      (Protection::WRITE, PROT_WRITE),
      (Protection::EXECUTE, PROT_EXEC),
    ];

    for &(prot, unix_flag) in &prots {
      if self.contains(prot) {
        result |= unix_flag;
      }
    }

    result
  }
}

pub fn page_size() -> usize {
  unsafe { ::libc::sysconf(::libc::_SC_PAGESIZE) as usize }
}

pub fn set_protection(base: *const u8, size: usize, protection: Protection) -> Result<()> {
  let result =
    unsafe { ::libc::mprotect(base as *mut ::libc::c_void, size, protection.to_native()) };

  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(io::Error::last_os_error())),
  }
}

pub fn lock(base: *const u8, size: usize) -> Result<()> {
  let result = unsafe { ::libc::mlock(base as *const ::libc::c_void, size) };
  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(io::Error::last_os_error())),
  }
}

pub fn unlock(base: *const u8, size: usize) -> Result<()> {
  let result = unsafe { ::libc::munlock(base as *const ::libc::c_void, size) };
  match result {
    0 => Ok(()),
    _ => Err(Error::SystemCall(io::Error::last_os_error())),
  }
}
