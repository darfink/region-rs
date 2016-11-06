use Protection;

/// A descriptor for a memory region
///
/// This type acts as a POD-type, i.e it has no functionality but merely
/// stores region information.
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
    /// Returns the region's lower bound.
    pub fn lower(&self) -> usize {
        self.base as usize
    }

    /// Returns the region's upper bound.
    pub fn upper(&self) -> usize {
        self.lower() + self.size
    }
}
