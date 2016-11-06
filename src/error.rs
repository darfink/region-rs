use std::fmt;

/// Possible errors when altering region properties.
#[derive(Debug)]
pub enum Error {
    /// The supplied address is null.
    Null,
    /// The queried memory is free.
    Freed,
    /// Invalid regex group match.
    ProcfsGroup,
    /// Procfs/maps could not be opened.
    ProcfsIo(::std::io::Error),
    /// Invalid regex group count.
    ProcfsMatches,
    /// Failed to parse number ranges.
    ProcfsConvert(::std::num::ParseIntError),

    // Specific for Windows
    VirtualLock(::errno::Errno),
    VirtualUnlock(::errno::Errno),
    VirtualProtect(::errno::Errno),
    VirtualQuery(::errno::Errno),

    /// Call to `mach_vm_region` failed (kernel error code).
    MachRegion(::libc::c_int),

    // Specific for Unix
    Mprotect(::errno::Errno),
    Munlock(::errno::Errno),
    Mlock(::errno::Errno),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let str = match *self {
            Error::Null => "Address must not be null",
            Error::Freed => "Address does not contain allocated memory",
            Error::ProcfsGroup => "Capture group is empty",
            Error::ProcfsIo(..) => "Procfs could not be opened",
            Error::ProcfsMatches => "Invalid capture group count",
            Error::ProcfsConvert(..) => "Failed to convert address to integral",
            Error::VirtualLock(..) => "Call 'VirtualLock' failed",
            Error::VirtualUnlock(..) => "Call 'VirtualUnlock' failed",
            Error::VirtualProtect(..) => "Call 'VirtualProtect' failed",
            Error::VirtualQuery(..) => "Call 'VirtualQuery' failed",
            Error::MachRegion(..) => "Call 'mach_vm_region' failed",
            Error::Mprotect(..) => "Call 'mprotect' failed",
            Error::Munlock(..) => "Call 'munlock' failed",
            Error::Mlock(..) => "Call 'mlock' failed",
        };

        write!(out, "{}", str)
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        "memory region error"
    }
}
