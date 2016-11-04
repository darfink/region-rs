extern crate regex;

use std::collections::HashMap;

use Error;
use Protection;
use Region;

/// Parses flags from /proc/[pid]/maps (e.g 'r--p')
fn parse_procfs_flags(protection: &str) -> (Protection, bool) {
    let shared = protection.ends_with('s');
    let protection = 0;

    if protection.find('r') != None {
        protection |= Protection::Read;
    }

    if protection.find('w') != None {
        protection |= Protection::Read;
    }

    if protection.find('x') != None {
        protection |= Protection::Read;
    }

    (protection as Protection, shared)
}

/// Parses a region from /proc/[pid]/maps (i.e a single line)
fn parse_procfs_region(input: &str) -> Result<Region, Error> {
    use self::regex::Regex;

    lazy_static! {
        static ref RE: Regex = Regex::new("^([0-9a-fA-F]+)-([0-9a-fA-F]+) (\\w|-){4}").unwrap();
    }

    match RE.captures(input) {
        Some(ref captures) if captures.len() == 3 => {
            let region_boundary: Vec<usize> = try!(captures.iter()
                .take(2)
                .map(|subcapture| {
                    subcapture.ok_or(Error::ProcfsGroup).and_then(|address| {
                        usize::from_str_radix(address, 16).map_err(Error::ProcfsConvert)
                    })
                })
                .collect());

            let (lower, upper) = (region_boundary[0], region_boundary[1]);
            let (protection, shared) = parse_procfs_flags(captures.at(2).unwrap());

            Ok(Region {
                base: lower as *mut u8,
                guarded: false,
                protection: protection,
                shared: shared,
                size: upper - lower,
            });
        }
        _ => Err(Error::ProcfsMatches),
    }
}

pub fn get_region(address: *const u8) -> Result<Region> {
    use std::fs::File;
    use std::io::{BufReader, BufRead};

    let address = address as usize;
    let file = try!(File::open("/proc/self/maps").map_err(Error::ProcfsIo));
    let reader = BufReader::new(&file).lines();

    for line in reader {
        let line = try!(line.map_err(Error::ProcfsIo));
        let query = try!(parse_procfs_region(line));

        if query.base >= address && address < query.upper() {
            return query;
        }
    }

    Err(Error::ProcfsRange)
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

        assert_eq!(region.base, 0x400000);
        assert_eq!(region.guarded, false);
        assert_eq!(region.protection, Protection::ReadExecute);
        assert_eq!(region.shared, true);
        assert_eq!(region.size, 0x9000);
    }
}
