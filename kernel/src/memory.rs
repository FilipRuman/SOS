use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x86_64::PhysAddr;
use x86_64::{VirtAddr, structures::paging::PageTable};

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}
pub unsafe fn init_mem(
    physical_memory_offset_u64: u64,
    memory_map: bootloader_api::info::MemoryRegions,
) {
    let phys_mem_offset = VirtAddr::new(physical_memory_offset_u64);

    let level_4_table = unsafe { active_level_4_table(phys_mem_offset) };
    MAPPER
        .init_once(|| Mutex::new(unsafe { OffsetPageTable::new(level_4_table, phys_mem_offset) }));
    FRAMES.init_once(|| {
        Mutex::new(StaticFrames {
            memory_map: memory_map.into(),
            next: 0,
        })
    });
}

use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, PhysFrame, Size4KiB};

pub static FRAMES: OnceCell<Mutex<StaticFrames>> = OnceCell::uninit();
pub static MAPPER: OnceCell<Mutex<OffsetPageTable<'static>>> = OnceCell::uninit();
/// A FrameAllocator that returns usable frames from the bootloader's memory map.


pub struct StaticFrames {
    memory_map: &'static mut [MemoryRegion],

    next: usize,
}
impl StaticFrames {
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();

        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}
pub struct StaticFrameAllocator {}

unsafe impl FrameAllocator<Size4KiB> for StaticFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let mut allocator = FRAMES
            .get()
            .expect("static frames were not yet initialized!")
            .lock();
        let frame = allocator.usable_frames().nth(allocator.next);
        allocator.next += 1;

        frame
    }
}

pub fn init_static(memory_map: bootloader_api::info::MemoryRegions) {
    let static_version = StaticFrames {
        memory_map: memory_map.into(),
        next: 0,
    };
    FRAMES.init_once(|| Mutex::new(static_version));
}
