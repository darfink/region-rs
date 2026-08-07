//! Error types and utilities.

use core::fmt;

/// The result type used by this library.
pub type Result<T> = core::result::Result<T, Error>;

/// A collection of possible errors.
#[derive(Debug)]
pub enum Error {
  /// The queried memory is unmapped.
  ///
  /// This does not necessarily mean that the memory region is available for
  /// allocation. Besides OS-specific semantics, queried addresses outside of a
  /// process' address range are also identified as unmapped regions.
  UnmappedRegion,
  /// A supplied parameter is invalid.
  InvalidParameter(&'static str),
  /// A procfs region could not be parsed.
  ProcfsInput(alloc::string::String),
  /// A system call failed.
  ///
  /// The contained value is the operating system's error code (for example
  /// `errno` on Unix-like platforms, or `GetLastError` on Windows).
  SystemCall(i32),
  /// A macOS kernel call failed.
  MachCall(i32),
}

impl fmt::Display for Error {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::UnmappedRegion => write!(f, "Queried memory is unmapped"),
      Error::InvalidParameter(param) => write!(f, "Invalid parameter value: {param}"),
      Error::ProcfsInput(input) => write!(f, "Invalid procfs input: {input}"),
      Error::SystemCall(code) => write!(f, "System call failed: {code}"),
      Error::MachCall(code) => write!(f, "macOS kernel call failed: {code}"),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl Error {
  /// Creates a [`Error::SystemCall`] from the current operating system error.
  #[inline]
  pub(crate) fn last_os_error() -> Self {
    Self::SystemCall(last_os_error_code())
  }
}

#[inline]
fn last_os_error_code() -> i32 {
  #[cfg(unix)]
  {
    // Prefer the portable errno accessor when available.
    #[cfg(any(target_os = "linux", target_os = "hurd", target_os = "fuchsia"))]
    unsafe {
      *libc::__errno_location()
    }

    #[cfg(any(
      target_os = "android",
      target_os = "macos",
      target_os = "ios",
      target_os = "freebsd",
      target_os = "openbsd",
      target_os = "netbsd",
      target_os = "dragonfly"
    ))]
    unsafe {
      *libc::__error()
    }

    #[cfg(any(target_os = "illumos", target_os = "solaris"))]
    unsafe {
      *libc::___errno()
    }

    #[cfg(not(any(
      target_os = "linux",
      target_os = "android",
      target_os = "hurd",
      target_os = "fuchsia",
      target_os = "macos",
      target_os = "ios",
      target_os = "freebsd",
      target_os = "openbsd",
      target_os = "netbsd",
      target_os = "dragonfly",
      target_os = "illumos",
      target_os = "solaris"
    )))]
    {
      0
    }
  }

  #[cfg(windows)]
  {
    unsafe { windows_sys::Win32::Foundation::GetLastError() as i32 }
  }

  #[cfg(not(any(unix, windows)))]
  {
    0
  }
}
