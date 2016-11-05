use Protection;

/// A descriptor for a memory region
#[derive(Debug, Clone, Copy)]
pub struct Region {
    /// Base address of the region
    pub base: *mut u8,
    /// Whether the region is guarded or not
    pub guarded: bool,
    /// Protection of the region
    pub protection: Protection::Flag,
    /// Whether the region is shared or not
    pub shared: bool,
    /// Size of the region (multiple of page size)
    pub size: usize,
}

impl Region {
    pub fn lower(&self) -> usize {
        self.base as usize
    }

    pub fn upper(&self) -> usize {
        self.lower() + self.size
    }
}
