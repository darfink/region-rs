//! Error types and utilities.

use std::error::Error as StdError;
use std::{fmt, io};

/// The result type used by this library.
pub type Result<T> = ::std::result::Result<T, Error>;

/// A collection of possible errors.
#[derive(Debug)]
pub enum Error {
  /// The supplied address is null.
  Null,
  /// The supplied address range is empty.
  EmptyRange,
  /// The queried memory is free.
  Free,
  /// Invalid procfs input.
  ProcfsInput,
  /// A system call failed.
  SystemCall(io::Error),
  /// A macOS kernel call failed
  MachRegion(::libc::c_int),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::Null => write!(f, "Address must not be null"),
      Error::EmptyRange => write!(f, "Address range must be larger than zero"),
      Error::Free => write!(f, "Address does not contain allocated memory"),
      Error::ProcfsInput => write!(f, "Invalid procfs input"),
      Error::SystemCall(ref error) => write!(f, "System call failed: {}", error),
      Error::MachRegion(code) => write!(f, "macOS kernel call failed: {}", code),
    }
  }
}

impl StdError for Error {}
