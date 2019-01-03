use {Error, Protection, Region, Result};

/// Parses flags from /proc/[pid]/maps (e.g 'r--p')
fn parse_procfs_flags(protection: &str) -> (Protection, bool) {
  const MAPPING: &[(char, Protection)] = &[
    ('r', Protection::Read),
    ('w', Protection::Write),
    ('x', Protection::Execute),
  ];

  let result = MAPPING
    .iter()
    .fold(Protection::None, |acc, &(ident, prot)| {
      acc | protection
        .find(ident)
        .map(|_| prot)
        .unwrap_or(Protection::None)
    });

  (result, protection.ends_with('s'))
}

/// Parses a region from /proc/[pid]/maps (i.e a single line)
fn parse_procfs_region(input: &str) -> Option<Region> {
  let mut parts = input.split_whitespace();
  let mut memory = parts
    .next()?
    .split('-')
    .filter_map(|value| usize::from_str_radix(value, 16).ok());
  let (lower, upper) = (memory.next()?, memory.next()?);

  let flags = parts.next()?;
  let (protection, shared) = parse_procfs_flags(flags);

  Some(Region {
    base: lower as *const _,
    size: upper - lower,
    guarded: false,
    protection,
    shared,
  })
}

pub fn get_region(address: *const u8) -> Result<Region> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};

  let address = address as usize;
  let file = File::open("/proc/self/maps").map_err(Error::SystemCall)?;
  let reader = BufReader::new(&file).lines();

  for line in reader {
    let line = line.map_err(Error::SystemCall)?;
    let region = parse_procfs_region(&line).ok_or(Error::ProcfsInput)?;
    let region_base = region.base as usize;

    if address >= region_base && address < region_base + region.size {
      return Ok(region);
    }
  }

  Err(Error::FreeMemory)
}

#[cfg(test)]
mod tests {
  use super::{parse_procfs_flags, parse_procfs_region};
  use Protection;

  #[test]
  fn parse_flags() {
    assert_eq!(parse_procfs_flags("r--s"), (Protection::Read, true));
    assert_eq!(parse_procfs_flags("rw-p"), (Protection::ReadWrite, false));
    assert_eq!(parse_procfs_flags("r-xs"), (Protection::ReadExecute, true));
    assert_eq!(
      parse_procfs_flags("rwxs"),
      (Protection::ReadWriteExecute, true)
    );
    assert_eq!(parse_procfs_flags("--xp"), (Protection::Execute, false));
    assert_eq!(parse_procfs_flags("-w-s"), (Protection::Write, true));
  }

  #[test]
  fn parse_region() {
    let line = "00400000-00409000 r-xs 00000000 08:00 16088 /usr/bin/head";
    let region = parse_procfs_region(line).unwrap();

    assert_eq!(region.base, 0x400000 as *mut u8);
    assert_eq!(region.guarded, false);
    assert_eq!(region.protection, Protection::ReadExecute);
    assert_eq!(region.shared, true);
    assert_eq!(region.size, 0x9000);
  }
}
