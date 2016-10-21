extern crate libc;

use std::collections::HashMap;

use Protection;

pub fn page_size() -> usize {
    lazy_static! {
        static ref PAGESIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    }

    *PAGESIZE
}

impl<'t> From<&'t str> for Protection {
    fn from(protection: &str) -> Self {
        lazy_static! {
            static ref MAP: HashMap<char, Protection> = map![
                'r' => Protection::Read,
                'w' => Protection::Write,
                'x' => Protection::Execute
            ];
        }

        (*MAP).iter().fold(Protection::None, |prot, (key, val)| {
            if protection.find(*key).is_some() {
                prot | *val
            } else {
                prot
            }
        })
    }
}

impl From<Protection> for ::libc::c_int {
    fn from(protection: Protection) -> Self {
        use libc::{PROT_NONE, PROT_READ, PROT_WRITE, PROT_EXEC};

        lazy_static! {
            static ref MAP: HashMap<Protection, ::libc::c_int> = map![
                Protection::Read => PROT_READ,
                Protection::Write => PROT_WRITE,
                Protection::Execute => PROT_EXEC
            ];
        }

        (*MAP).iter().fold(PROT_NONE, |prot, (key, val)| {
            if protection.contains(*key) {
                prot | *val
            } else {
                prot
            }
        })
    }
}
