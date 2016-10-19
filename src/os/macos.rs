extern crate mach;

use std::collections::HashMap;
use self::mach::vm_prot::*;

use Protection;

impl From<vm_prot_t> for Protection {
    fn from(protection: vm_prot_t) -> Self {
        lazy_static! {
            static ref MAP: HashMap<vm_prot_t, Protection> = map![
                VM_PROT_READ => Protection::Read,
                VM_PROT_WRITE => Protection::Write,
                VM_PROT_EXECUTE => Protection::Execute
            ];
        }

        (*MAP).iter().fold(Protection::None, |prot, (key, val)| {
            if (protection & *key) == *key { prot | *val } else { prot }
        })
    }
}
