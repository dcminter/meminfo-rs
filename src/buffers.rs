use procfs::{Current, Meminfo};

/// The latest progress of a cache entry
#[derive(Debug)]
pub struct MemRange {
    /// The current value of the cache entry
    pub current: u64,

    /// The highest value seen for the cache entry so far
    pub highest: u64,
}

/// The cache entries that this utility is tracking
#[derive(Debug)]
pub struct MemCounts {
    /// Memory which is waiting to get written back to the disk
    pub dirty: MemRange,

    /// Memory which is actively being written back to the disk
    pub writeback: MemRange,
}

fn memory_count_update(meminfo: &Meminfo, mc: &mut MemCounts) {
    // Update the dirty stats
    let dirty_range = &mut mc.dirty;
    if meminfo.dirty > dirty_range.highest {
        dirty_range.current = meminfo.dirty;
        dirty_range.highest = meminfo.dirty;
    } else {
        dirty_range.current = meminfo.dirty;
    };

    // Update the writeback stats
    let writeback_range = &mut mc.writeback;
    if meminfo.writeback > writeback_range.highest {
        writeback_range.current = meminfo.writeback;
        writeback_range.highest = meminfo.writeback;
    } else {
        writeback_range.current = meminfo.writeback;
    };
}

pub fn meminfo_reader(mc: &mut MemCounts) {
    match &Meminfo::current() {
        Ok(meminfo) => {
            memory_count_update(meminfo, mc);
        }
        Err(error) => {
            eprintln!("Error reading memory info: {}", error);
        }
    }
}
