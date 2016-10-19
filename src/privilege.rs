#![allow(non_upper_case_globals)]

bitflags! {
    #[repr(C)]
    #[derive(Default)]
    pub flags Protection: i32 {
        // These should preferably map to (Unix) PROT_* flags
        const None             = (0 << 0),
        const Execute          = (1 << 0),
        const Write            = (1 << 1),
        const Read             = (1 << 2),

        // Shorthand declarations
        const ReadWrite        = Self::Read.bits | Self::Write.bits,
        const ReadExecute      = Self::Read.bits | Self::Execute.bits,
        const ReadWriteExecute = Self::Execute.bits | Self::Read.bits | Self::Write.bits,
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Access {
    Initial,
    Previous,
    Type(Protection),
}

impl From<Protection> for Access {
    fn from(protection: Protection) -> Self {
        Access::Type(protection)
    }
}
