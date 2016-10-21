#[cfg(unix)]
pub mod unix;

#[cfg(unix)]
pub use self::unix::*;

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

pub fn truncate_page(address: usize) -> usize {
    address & !(page_size() - 1)
}

pub fn round_page(address: usize) -> usize {
    let page_size = page_size();
    (address + page_size - 1) & !(page_size - 1)
}

#[test]
fn test_page_alignment() {
    let page_size = page_size();
    assert_eq!(truncate_page(1), 0);
    assert_eq!(round_page(1), page_size);
}
