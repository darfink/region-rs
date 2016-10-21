use std::fmt;

/// Possible errors when creating a map.
#[derive(Debug)]
pub enum Error {
    Null,

    // Specific for Linux
    ProcfsGroup,
    ProcfsIo(::std::io::Error),
    ProcfsMatches,
    ProcfsParse(::std::num::ParseIntError),
    ProcfsRange,

    // Specific for Windows
    VirtualQuery(::errno::Errno),
    VirtualProtect(::errno::Errno),

    // Specific for macOS
    MachRegion(::libc::c_int),

    // Specific for Unix
    Mprotect(::errno::Errno),
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let str = match *self {
            Error::Null               => "Invalid address",
            Error::ProcfsGroup        => "Empty match group",
            Error::ProcfsIo(..)       => "Failed to open procfs",
            Error::ProcfsMatches      => "Invalid match count",
            Error::ProcfsParse(..)    => "Failed to parse address",
            Error::ProcfsRange        => "Address range not found",
            Error::VirtualQuery(..)   => "Call 'VirtualQuery' failed",
            Error::VirtualProtect(..) => "Call 'VirtualProtect' failed",
            Error::MachRegion(..)     => "Call 'mach_vm_region' failed",
            Error::Mprotect(..)       => "Call 'mprotect' failed",
        };

        write!(out, "{}", str)
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str { "memory region error" }
}
