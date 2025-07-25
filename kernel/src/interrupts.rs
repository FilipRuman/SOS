use x86_64::structures::paging::{FrameAllocator, Mapper, Size4KiB};

pub mod apic;
pub mod pic;
pub fn init(rsdp: usize) {
    pic::disable_pic();
    apic::init(rsdp);
}
