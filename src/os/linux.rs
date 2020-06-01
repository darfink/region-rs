use crate::{Error, Protection, Region, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter;
use take_until::TakeUntilExt;

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  let file = File::open("/proc/self/maps").map_err(Error::SystemCall)?;
  let mut reader = BufReader::new(file);
  let mut line = String::new();

  let upper_bound = (origin as usize).saturating_add(size);
  let iterator = iter::from_fn(move || {
    line.clear();
    match reader.read_line(&mut line) {
      Ok(0) => None,
      Ok(_) => Some(parse_procfs_line(&line).ok_or_else(|| Error::ProcfsInput(line.clone()))),
      Err(error) => Some(Err(Error::SystemCall(error))),
    }
  })
  .skip_while(move |res| matches!(res, Ok(region) if region.as_range().end <= origin as usize))
  .take_while(move |res| !matches!(res, Ok(region) if region.as_range().start >= upper_bound))
  .take_until(|res| res.is_err())
  .fuse();
  Ok(iterator)
}

/// Parses flags from /proc/[pid]/maps (e.g 'r--p').
fn parse_procfs_flags(protection: &str) -> (Protection, bool) {
  const MAPPINGS: &[Protection] = &[Protection::READ, Protection::WRITE, Protection::EXECUTE];

  let result = protection
    .chars()
    .zip(MAPPINGS.iter())
    .filter(|(c, _)| *c != '-')
    .fold(Protection::NONE, |acc, (_, prot)| acc | *prot);

  (result, protection.ends_with('s'))
}

/// Parses a line from /proc/[pid]/maps.
fn parse_procfs_line(input: &str) -> Option<Region> {
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

#[cfg(test)]
mod tests {
  use super::{parse_procfs_flags, parse_procfs_line};
  use crate::Protection;

  #[test]
  fn procfs_flags_are_parsed() {
    let rwx = Protection::READ_WRITE_EXECUTE;

    assert_eq!(parse_procfs_flags("r--s"), (Protection::READ, true));
    assert_eq!(parse_procfs_flags("rw-p"), (Protection::READ_WRITE, false));
    assert_eq!(parse_procfs_flags("r-xs"), (Protection::READ_EXECUTE, true));
    assert_eq!(parse_procfs_flags("rwxs"), (rwx, true));
    assert_eq!(parse_procfs_flags("--xp"), (Protection::EXECUTE, false));
    assert_eq!(parse_procfs_flags("-w-s"), (Protection::WRITE, true));
  }

  #[test]
  fn procfs_regions_are_parsed() {
    let line = "00400000-00409000 r-xs 00000000 08:00 16088 /usr/bin/head";
    let region = parse_procfs_line(line).unwrap();

    assert_eq!(region.as_ptr(), 0x400000 as *mut ());
    assert_eq!(region.protection(), Protection::READ_EXECUTE);
    assert_eq!(region.is_guarded(), false);
    assert_eq!(region.len(), 0x9000);
    assert!(region.is_shared());
  }
}
