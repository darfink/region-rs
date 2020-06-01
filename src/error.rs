//! Error types and utilities.

use std::error::Error as StdError;
use std::{fmt, io};

/// The result type used by this library.
pub type Result<T> = std::result::Result<T, Error>;

/// A collection of possible errors.
#[derive(Debug)]
pub enum Error {
  /// The queried memory is umapped.
  ///
  /// This does not necessarily mean that the memory region can be allocated,
  /// queried addresses outside of a process' adress range are also identified as
  /// unmapped regions.
  UnmappedRegion,
  /// A supplied parameter is invalid.
  InvalidParameter(&'static str),
  /// Invalid procfs input.
  ProcfsInput(String),
  /// A system call failed.
  SystemCall(io::Error),
  /// A macOS kernel call failed
  MachCall(libc::c_int),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::UnmappedRegion => write!(f, "Queried memory is unmapped"),
      Error::InvalidParameter(param) => write!(f, "Invalid parameter value: {}", param),
      Error::ProcfsInput(ref input) => write!(f, "Invalid procfs input: {}", input),
      Error::SystemCall(ref error) => write!(f, "System call failed: {}", error),
      Error::MachCall(code) => write!(f, "macOS kernel call failed: {}", code),
    }
  }
}

impl StdError for Error {}
