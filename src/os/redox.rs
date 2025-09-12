use crate::{Error, Region, Result};

pub struct QueryIter {
  upper_bound: usize,
}

// redox os doesn't have this feature yet
impl QueryIter {
  pub fn new(_origin: *const (), _size: usize) -> Result<Self> {
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
