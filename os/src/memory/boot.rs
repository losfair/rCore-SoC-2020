use super::{LockedPagePool, Mapping, PageTableEntryFlags, Segment, SegmentBacking};
use crate::error::*;
use crate::layout;
use crate::process::ThreadToken;

pub unsafe fn remap_kernel(pool: LockedPagePool, token: &ThreadToken) -> KernelResult<Mapping> {
    let mut mapping = Mapping::new_without_kernel_region(pool, token)?;
    let ksegs: &[Segment] = &[
        Segment {
            range: layout::text_start().vpn()..layout::rodata_start().vpn(),
            backing: SegmentBacking::Linear {
                phys_start: layout::text_start()
                    .to_phys()
                    .expect("remap_kernel: bad text_start")
                    .ppn(),
            },
            flags: PageTableEntryFlags::VALID
                | PageTableEntryFlags::READABLE
                | PageTableEntryFlags::EXECUTABLE,
        },
        Segment {
            range: layout::rodata_start().vpn()..layout::data_start().vpn(),
            backing: SegmentBacking::Linear {
                phys_start: layout::rodata_start()
                    .to_phys()
                    .expect("remap_kernel: bad rodata_start")
                    .ppn(),
            },
            flags: PageTableEntryFlags::VALID | PageTableEntryFlags::READABLE,
        },
        Segment {
            range: layout::data_start().vpn()..layout::ram_end().vpn(),
            backing: SegmentBacking::Linear {
                phys_start: layout::data_start()
                    .to_phys()
                    .expect("remap_kernel: bad data_start")
                    .ppn(),
            },
            flags: PageTableEntryFlags::VALID
                | PageTableEntryFlags::READABLE
                | PageTableEntryFlags::WRITABLE,
        },
    ];
    for seg in ksegs {
        mapping.map_segment(seg, token)?;
    }
    mapping.activate_thread(token);
    Ok(mapping)
}
