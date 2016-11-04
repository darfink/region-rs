#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use self::unix::*;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use self::linux::*;

pub fn page_floor(address: usize) -> usize {
    address & !(page_size() - 1)
}

pub fn page_ceil(address: usize) -> usize {
    let page_size = page_size();
    (address + page_size - 1) & !(page_size - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_value() {
        let pz = page_size();

        assert!(pz > 0);
        assert!(pz % 2 == 0);
    }

    #[test]
    fn page_rounding() {
        let pz = page_size();

        // Truncates down
        assert_eq!(page_floor(1), 0);
        assert_eq!(page_floor(pz), pz);
        assert_eq!(page_floor(pz + 1), pz);

        // Rounds up
        assert_eq!(page_ceil(0), 0);
        assert_eq!(page_ceil(1), pz);
        assert_eq!(page_ceil(pz), pz);
        assert_eq!(page_ceil(pz + 1), pz * 2);
    }
}
