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

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::get_region;
