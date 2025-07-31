use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use bootloader_api::BootInfo;
use bootloader_api::info::{MemoryRegion, MemoryRegionKind};
use conquer_once::spin::OnceCell;
use log::debug;
use spin::Mutex;
use x86_64::PhysAddr;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::{VirtAddr, structures::paging::PageTable};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 10000 * 1024; // 10000 KiB

pub const ACPI_MEMORY_SIZE: usize = 4 * 8 * 1024 * 2;
pub const ACPI_START_ADDRESS: usize = HEAP_START + HEAP_SIZE + 1;

pub const PER_AP_STACK_STACK_SIZE: usize = 16383; // 16 KiB per core

pub const AP_STACK_MEMORY_START: usize = ACPI_START_ADDRESS + ACPI_MEMORY_SIZE; // skip 1 page - 4 KiB

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> (&'static mut PageTable, u64) {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address().as_u64();

    let virt = physical_memory_offset + phys;
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    (unsafe { &mut *page_table_ptr }, phys)
}
pub unsafe fn init_mem(
    physical_memory_offset_u64: u64,
    memory_map: bootloader_api::info::MemoryRegions,
) -> u64 {
    let phys_mem_offset = VirtAddr::new(physical_memory_offset_u64);

    let (level_4_table_virt, level_4_table_phys_address) =
        unsafe { active_level_4_table(phys_mem_offset) };
    MAPPER.init_once(|| {
        Mutex::new(unsafe { OffsetPageTable::new(level_4_table_virt, phys_mem_offset) })
    });
    FRAMES.init_once(|| {
        Mutex::new(StaticFrames {
            memory_map: memory_map.into(),
            next: 0,
        })
    });

    level_4_table_phys_address
}
/// # Safety
/// invalid memory address could lead to unexpected behavior
pub unsafe fn map_memory(start: usize, size: usize, flags: PageTableFlags) {
    let mut frame_allocator = StaticFrameAllocator {};
    let mut mapper = MAPPER.get().expect("Memory was not yet initialized").lock();

    let page_range = {
        let heap_start = VirtAddr::new(start as u64);
        debug!("start: {heap_start:?}");
        let heap_end = heap_start + size as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };
    for page in page_range {
        debug!("map page: {page:?}");
        let frame = frame_allocator.allocate_frame().unwrap();
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)
                .unwrap()
                .flush()
        };
    }
}

use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
};

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
