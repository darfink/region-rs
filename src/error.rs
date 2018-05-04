//! Error types and utilities.

use std::io;

/// The result type used by this library.
pub type Result<T> = ::std::result::Result<T, Error>;

/// A collection of possible errors.
#[derive(Debug, Fail)]
pub enum Error {
  /// The supplied address is null.
  #[fail(display = "address must not be null")]
  Null,
  /// The queried memory is free.
  #[fail(display = "address does not contain allocated memory")]
  Free,
  /// Invalid procfs input.
  #[fail(display = "invalid procfs source input")]
  ProcfsInput,
  /// A proc fs IO operation failed.
  #[fail(display = "{}", _0)]
  ProcfsIo(#[cause] io::Error),
  /// A system call failed.
  #[fail(display = "system call failed with: {}", _0)]
  SystemCall(::errno::Errno),
  /// A macOS kernel call failed
  #[fail(display = "macOS kernel call failed with: {}", _0)]
  MachRegion(::libc::c_int),
}

impl From<io::Error> for Error {
  fn from(error: io::Error) -> Self {
    Error::ProcfsIo(error)
  }
}
