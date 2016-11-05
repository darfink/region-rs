use std::fmt;

/// Possible errors when creating a map.
#[derive(Debug)]
pub enum Error {
    Null,
    Freed,

    // Specific for Linux
    ProcfsGroup,
    ProcfsIo(::std::io::Error),
    ProcfsMatches,
    ProcfsConvert(::std::num::ParseIntError),
    ProcfsRange,

    // Specific for Windows
    VirtualLock(::errno::Errno),
    VirtualUnlock(::errno::Errno),
    VirtualProtect(::errno::Errno),
    VirtualQuery(::errno::Errno),

    // Specific for macOS
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
            Error::ProcfsRange => "Address range not found",
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
