#![feature(associated_consts)]
#![feature(step_by)]

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate bitflags;
extern crate libc;
extern crate errno;

pub use error::Error;
pub use privilege::{Protection, Access};
pub use region::*;

#[macro_use]
mod macros;
mod error;
mod os;
mod privilege;
mod region;

#[cfg(test)]
mod tests {
    use std::mem;
    use super::*;

    #[test]
    fn it_works() {
        let ret5: Vec<u8> = vec![0xB8, 0x05, 0x00, 0x00, 0x00, 0xC3];
        let mut region = Region::with_size(&ret5[0] as *const u8, ret5.len()).unwrap();

        region.exec_with_prot(Protection::ReadWriteExecute, || {
            let x: extern fn() -> i32 = unsafe { mem::transmute(&ret5[0]) };
            println!("Result: {}", x());
        }).unwrap();
    }
}
