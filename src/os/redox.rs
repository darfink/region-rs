use crate::{Error, Region, Result};

pub struct QueryIter {
  upper_bound: usize,
}

impl QueryIter {
  pub fn new(origin: *const (), size: usize) -> Result<Self> {
    let _ = (origin, size);
    // Redox does not yet expose a process memory map query API comparable to
    // the other platforms supported by this crate.
    Err(Error::UnmappedRegion)
  }

  pub fn upper_bound(&self) -> usize {
    self.upper_bound
  }
}

impl Iterator for QueryIter {
  type Item = Result<Region>;

  fn next(&mut self) -> Option<Self::Item> {
    None
  }
}
