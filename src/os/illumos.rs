use crate::{Error, Protection, Region, Result};
use std::fs::File;
use std::io::Read;
use take_until::TakeUntilExt;

// As per proc(4), the file "/proc/$PID/map" contains an array of C structs of
// type "prmap_t".  The layout of this struct, and thus this file, is a stable
// interface.
#[repr(C)]
struct PrMap {
  pr_vaddr: *const (),
  pr_size: usize,
  pr_mapname: [i8; 64],
  pr_offset: isize,
  pr_mflags: i32,
  pr_pagesize: i32,
  pr_shmid: i32,
  _pr_filler: [i32; 1],
}

const PRMAP_SIZE: usize = std::mem::size_of::<PrMap>();

// These come from <sys/procfs.h>, describing bits in the pr_mflags member:
const MA_EXEC: i32 = 0x1;
const MA_WRITE: i32 = 0x2;
const MA_READ: i32 = 0x4;
const MA_SHARED: i32 = 0x8;

pub struct PrMapIterator {
  buf: Vec<u8>,
  idx: usize,
}

impl Iterator for PrMapIterator {
  type Item = Result<Region>;

  fn next(&mut self) -> Option<Result<Region>> {
    let (pfx, maps, sfx) = unsafe { self.buf.align_to::<PrMap>() };

    if !pfx.is_empty() || !sfx.is_empty() {
      panic!(
        "data was not aligned ({}; {}/{}/{})?",
        self.buf.len(),
        pfx.len(),
        maps.len(),
        sfx.len()
      );
    }

    if let Some(map) = maps.get(self.idx) {
      let mut protection = Protection::NONE;
      if map.pr_mflags & MA_READ != 0 {
        protection |= Protection::READ;
      }
      if map.pr_mflags & MA_WRITE != 0 {
        protection |= Protection::WRITE;
      }
      if map.pr_mflags & MA_EXEC != 0 {
        protection |= Protection::EXECUTE;
      }

      let reg = Region {
        base: map.pr_vaddr,
        protection,
        shared: map.pr_mflags & MA_SHARED != 0,
        size: map.pr_size,
        ..Default::default()
      };

      self.idx += 1;
      Some(Ok(reg))
    } else {
      None
    }
  }
}

pub fn query<T>(origin: *const T, size: usize) -> Result<impl Iterator<Item = Result<Region>>> {
  // Do not use a buffered reader here: as much as possible, we want a single
  // large read(2) call to the proc file.
  let mut file = File::open("/proc/self/map").map_err(Error::SystemCall)?;
  let mut buf: Vec<u8> = Vec::with_capacity(8 * PRMAP_SIZE);

  let sz = file.read_to_end(&mut buf).map_err(Error::SystemCall)?;
  if sz % PRMAP_SIZE != 0 {
    return Err(Error::ProcfsInput(format!(
      "file size {} is not a multiple of prmap_t size ({})",
      sz, PRMAP_SIZE
    )));
  }

  let upper_bound = (origin as usize).saturating_add(size);
  let iterator = PrMapIterator { buf, idx: 0 }
    .skip_while(move |res| matches!(res, Ok(reg) if reg.as_range().end <= origin as usize))
    .take_while(move |res| !matches!(res, Ok(reg) if reg.as_range().start >= upper_bound))
    .take_until(|res| res.is_err())
    .fuse();
  Ok(iterator)
}
