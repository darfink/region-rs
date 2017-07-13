use error::*;
use Protection;

use libc::{PROT_NONE, PROT_READ, PROT_WRITE, PROT_EXEC};

fn convert_to_native(protection: Protection::Flag) -> ::libc::c_int {
    let mut result = PROT_NONE;

    if protection.contains(Protection::Read) {
        result |= PROT_READ;
    }

    if protection.contains(Protection::Write) {
        result |= PROT_WRITE;
    }

    if protection.contains(Protection::Execute) {
        result |= PROT_EXEC;
    }

    result
}

pub fn page_size() -> usize {
    unsafe { ::libc::sysconf(::libc::_SC_PAGESIZE) as usize }
}

pub fn set_protection(base: *const u8, size: usize, protection: Protection::Flag) -> Result<()> {
    let result = unsafe {
        ::libc::mprotect(base as *mut ::libc::c_void,
                         size,
                         convert_to_native(protection))
    };

    match result {
        0 => Ok(()),
        _ => Err(ErrorKind::SystemCall(::errno::errno()).into()),
    }
}

pub fn lock(base: *const u8, size: usize) -> Result<()> {
    let result = unsafe { ::libc::mlock(base as *const ::libc::c_void, size) };
    match result {
        0 => Ok(()),
        _ => Err(ErrorKind::SystemCall(::errno::errno()).into()),
    }
}

pub fn unlock(base: *const u8, size: usize) -> Result<()> {
    let result = unsafe { ::libc::munlock(base as *const ::libc::c_void, size) };
    match result {
        0 => Ok(()),
        _ => Err(ErrorKind::SystemCall(::errno::errno()).into()),
    }
}
