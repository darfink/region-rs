extern crate regex;

use error::*;
use Protection;
use Region;

/// Parses flags from /proc/[pid]/maps (e.g 'r--p')
fn parse_procfs_flags(protection: &str) -> (Protection::Flag, bool) {
    let shared = protection.ends_with('s');
    let mut result = Protection::None;

    if protection.find('r') != None {
        result |= Protection::Read;
    }

    if protection.find('w') != None {
        result |= Protection::Write;
    }

    if protection.find('x') != None {
        result |= Protection::Execute;
    }

    (result as Protection::Flag, shared)
}

/// Parses a region from /proc/[pid]/maps (i.e a single line)
fn parse_procfs_region(input: &str) -> Result<Region> {
    use self::regex::Regex;

    lazy_static! {
        static ref RE: Regex = Regex::new(
          "^([0-9a-fA-F]+)-([0-9a-fA-F]+) (?P<prot>(?:\\w|-){4})").unwrap();
    }

    match RE.captures(input) {
        Some(ref captures) if captures.len() == 4 => {
            let region_boundary: Vec<usize> = captures.iter()
                .skip(1)
                .take(2)
                .map(|capture| usize::from_str_radix(capture.unwrap().as_str(), 16).unwrap())
                .collect();

            let (lower, upper) = (region_boundary[0], region_boundary[1]);
            let (protection, shared) = parse_procfs_flags(captures.name("prot").unwrap().as_str());

            Ok(Region {
                base: lower as *const _,
                guarded: false,
                protection: protection,
                shared: shared,
                size: upper - lower,
            })
        }
        _ => bail!(ErrorKind::ProcfsMatches),
    }
}

pub fn get_region(address: *const u8) -> Result<Region> {
    use std::fs::File;
    use std::io::{BufReader, BufRead};

    let address = address as usize;
    let file = File::open("/proc/self/maps")?;
    let reader = BufReader::new(&file).lines();

    for line in reader {
        let region = parse_procfs_region(&line?)?;
        let region_base = region.base as usize;

        if address >= region_base && address < region_base + region.size {
            return Ok(region);
        }
    }

    Err(ErrorKind::Free.into())
}

#[cfg(test)]
mod tests {
    use Protection;
    use super::{parse_procfs_flags, parse_procfs_region};

    #[test]
    fn parse_flags() {
        assert_eq!(parse_procfs_flags("r--s"), (Protection::Read, true));
        assert_eq!(parse_procfs_flags("rw-p"), (Protection::ReadWrite, false));
        assert_eq!(parse_procfs_flags("r-xs"), (Protection::ReadExecute, true));
        assert_eq!(parse_procfs_flags("rwxs"),
                   (Protection::ReadWriteExecute, true));
        assert_eq!(parse_procfs_flags("--xp"), (Protection::Execute, false));
        assert_eq!(parse_procfs_flags("-w-s"), (Protection::Write, true));
    }

    #[test]
    fn parse_region() {
        let region = parse_procfs_region("00400000-00409000 r-xs 00000000 08:00 16088 \
                                          /usr/bin/head")
            .unwrap();

        assert_eq!(region.base, 0x400000 as *mut u8);
        assert_eq!(region.guarded, false);
        assert_eq!(region.protection, Protection::ReadExecute);
        assert_eq!(region.shared, true);
        assert_eq!(region.size, 0x9000);
    }
}
