use core::sync::atomic::AtomicU64;

use conquer_once::noblock::OnceCell;
use x86_64::structures::paging::{FrameAllocator, Mapper, Size4KiB};

pub mod apic;
pub mod pic;

pub static TSC_HZ: OnceCell<u64> = OnceCell::uninit();
pub fn init(rsdp: usize) -> u8 {
    let tsc_ticks_per_ms = pic::calibrate_tsc();
    let tsc_freq_hz = (tsc_ticks_per_ms as f32 * 1000.0 / 1.6944444444) as u64;
    TSC_HZ.try_init_once(|| tsc_freq_hz);

    pic::disable_pic();
    apic::init(rsdp)
}
