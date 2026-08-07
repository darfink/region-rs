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
  ///
  /// With the `std` feature enabled, use [`Error::raw_os_error`],
  /// [`Error::as_io_error`], or `Into<std::io::Error>` for richer I/O errors.
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
      #[cfg(feature = "std")]
      Error::SystemCall(code) => {
        let err = std::io::Error::from_raw_os_error(*code);
        write!(f, "System call failed: {err}")
      }
      #[cfg(not(feature = "std"))]
      Error::SystemCall(code) => write!(f, "System call failed: {code}"),
      Error::MachCall(code) => write!(f, "macOS kernel call failed: {code}"),
    }
  }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "std")]
impl From<Error> for std::io::Error {
  #[inline]
  fn from(error: Error) -> Self {
    match error {
      Error::SystemCall(code) => Self::from_raw_os_error(code),
      other => Self::other(other),
    }
  }
}

impl Error {
  /// Creates a [`Error::SystemCall`] from the current operating system error.
  #[inline]
  pub(crate) fn last_os_error() -> Self {
    Self::SystemCall(last_os_error_code())
  }

  /// Returns the raw OS error code for system-call failures.
  #[inline]
  pub fn raw_os_error(&self) -> Option<i32> {
    match *self {
      Error::SystemCall(code) => Some(code),
      _ => None,
    }
  }

  /// Converts a system-call failure into a [`std::io::Error`].
  ///
  /// Returns [`None`] for non-system-call errors. Prefer
  /// [`Error::into_io_error`] when you own the value and want conversion for
  /// every variant.
  #[cfg(feature = "std")]
  #[inline]
  pub fn as_io_error(&self) -> Option<std::io::Error> {
    self.raw_os_error().map(std::io::Error::from_raw_os_error)
  }

  /// Converts this error into a [`std::io::Error`].
  ///
  /// System-call failures preserve the raw OS code. Other variants become
  /// [`std::io::Error::other`].
  #[cfg(feature = "std")]
  #[inline]
  pub fn into_io_error(self) -> std::io::Error {
    self.into()
  }
}

#[inline]
fn last_os_error_code() -> i32 {
  #[cfg(unix)]
  {
    #[cfg(any(target_os = "linux", target_os = "hurd", target_os = "fuchsia"))]
    unsafe {
      *libc::__errno_location()
    }

    // Android / NetBSD / OpenBSD expose `__errno`.
    #[cfg(any(target_os = "android", target_os = "netbsd", target_os = "openbsd"))]
    unsafe {
      *libc::__errno()
    }

    #[cfg(any(
      target_os = "macos",
      target_os = "ios",
      target_os = "freebsd",
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
