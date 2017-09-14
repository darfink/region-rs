#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::{
    page_size,
    get_region,
    set_protection,
    unlock,
    lock,
};

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::{
    page_size,
    set_protection,
    unlock,
    lock,
};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::get_region;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::get_region;
