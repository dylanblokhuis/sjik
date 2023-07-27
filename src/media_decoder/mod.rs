use crossbeam_utils::atomic::AtomicCell;

pub type MasterClock = AtomicCell<u32>;