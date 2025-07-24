use x86_64::structures::paging::{FrameAllocator, Mapper, Size4KiB};

pub mod apic;
pub mod pic;
pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    pic::disable_pic();
    apic::init(mapper, frame_allocator);
}
