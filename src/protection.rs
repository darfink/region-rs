#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
pub mod Protection {
    bitflags! {
        pub flags Flag: usize {
            const None             = (0 << 0),
            const Read             = (1 << 1),
            const Write            = (1 << 2),
            const Execute          = (1 << 3),
            const ReadExecute      = (Read.bits | Execute.bits),
            const ReadWrite        = (Read.bits | Write.bits),
            const ReadWriteExecute = (Read.bits | Write.bits | Execute.bits),
            const WriteExecute     = (Write.bits | Execute.bits),
        }
    }
}
