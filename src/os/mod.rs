#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::{get_region, lock, page_size, set_protection, unlock};

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::{lock, page_size, set_protection, unlock};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use self::macos::get_region;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub use self::linux::get_region;

#[cfg(target_os = "freebsd")]
mod freebsd;

#[cfg(target_os = "freebsd")]
pub use self::freebsd::get_region;
