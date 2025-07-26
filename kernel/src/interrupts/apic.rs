use core::{cell::OnceCell, ptr::NonNull};

use acpi::PhysicalMapping;
use crossbeam_queue::ArrayQueue;
use log::{debug, *};
use x86::apic::{self, ApicControl, ApicId};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB,
        mapper::{self, MapToError},
        page,
    },
};

fn io_apic() -> x86::apic::ioapic::IoApic {
    unsafe { x86::apic::ioapic::IoApic::new(IOAPIC_ADDR) }
}
fn xapic() -> x86::apic::xapic::XAPIC {
    let apic_address = VirtAddr::new(LOCAL_APIC_ADDR);
    let pointer = apic_address.as_mut_ptr();

    const APIC_LEN: usize = 1024;
    let slice: &'static mut [u32] = unsafe { core::slice::from_raw_parts_mut(pointer, APIC_LEN) };

    x86::apic::xapic::XAPIC::new(slice)
}

fn init_xapic() {
    let mut mapper = MAPPER.get().expect("memory was not yet initialized").lock();
    let mut frame_allocator = StaticFrameAllocator {};

    let apic_address = VirtAddr::new(LOCAL_APIC_ADDR);
    let page: x86_64::structures::paging::Page = Page::containing_address(apic_address);
    let frame = PhysFrame::containing_address(PhysAddr::new(LOCAL_APIC_ADDR));

    let flags = PageTableFlags::PRESENT | PageTableFlags::NO_CACHE | PageTableFlags::WRITABLE;
    unsafe {
        mapper
            .map_to(page, frame, flags, &mut frame_allocator)
            .expect("mapping memory for apic did not succeed")
            .flush()
    };

    let mut xapic = xapic();
    xapic.attach();
}

fn write_lapic(offset: u64, value: u32) {
    let reg = (LOCAL_APIC_ADDR + offset) as *mut u32;
    unsafe { core::ptr::write_volatile(reg, value) };
}
pub fn setup_xapic_timer() {
    let divide: u8 = 0b1011;
    // Divide config: 0b1011 = divide by 1
    write_lapic(x86::apic::xapic::XAPIC_TIMER_DIV_CONF as u64, divide as u32);

    let vector: u8 = 0x20;
    // LVT Timer: set mode = periodic (bit 17), and vector
    let lvt_value = (1 << 17) | (vector as u32); // Periodic | vector
    write_lapic(x86::apic::xapic::XAPIC_LVT_TIMER as u64, lvt_value);

    let init_count: u32 = 10_000_000;
    // Initial Count: how long until interrupt fires
    write_lapic(x86::apic::xapic::XAPIC_TIMER_INIT_COUNT as u64, init_count);
}

// WARN: THIS MIGHT NOT BE TRUE!

const LOCAL_APIC_ADDR: u64 = 0xFEE00000;
const IOAPIC_ADDR: usize = 0xFEC00000;
const IRQ_BASE: u8 = 32;

const TIMER_IRQ: u8 = 0; // maps to vector 32
const KEYBOARD_IRQ: u8 = 1; // maps to vector 33

use lazy_static::*;
use x86_64::structures::idt::InterruptStackFrame;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt[TIMER_IRQ + IRQ_BASE].set_handler_fn(timer_interrupt_handler);
        idt[KEYBOARD_IRQ + IRQ_BASE].set_handler_fn(keyboard_interrupt_handler);

        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}
const ACPI_MEMORY_SIZE: usize = 4 * 8 * 1024;
const ACPI_START_ADDRESS: usize = HEAP_START + HEAP_SIZE + ACPI_MEMORY_SIZE;

// assuming that size of page is 4KB
lazy_static! {
    pub static ref ACPI_PAGES: ArrayQueue<Page> = ArrayQueue::new(ACPI_MEMORY_SIZE / (4 * 1024));
}
#[derive(Clone)]
pub struct AcpiHandler {}
impl acpi::AcpiHandler for AcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        debug!("acpi: map_physical_region: size{size}");
        let mut frame_allocator = StaticFrameAllocator {};
        let mut mapper = MAPPER.get().expect("Memory was not yet initialized").lock();

        let page = ACPI_PAGES.pop().expect("not enough pages for acpi");
        let frame = PhysFrame::containing_address(PhysAddr::new(physical_address as u64));
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)
                .unwrap()
                .flush()
        };

        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(page.start_address().as_mut_ptr()).unwrap(),
                size,
                4 * 1024,
                AcpiHandler {},
            )
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        // TODO: not really needed, i don't use this part of memory for anything else
    }
}

pub const PIC_1_OFFSET: u8 = 32;

pub fn init(rsdp: usize) {
    init_xapic();

    map_memory_for_io_apic();

    IDT.load();

    debug!("IDT initialized!");
    setup_xapic_timer();

    let mut io_apic = io_apic();
    io_apic.enable(KEYBOARD_IRQ, 0);

    x86_64::instructions::interrupts::enable();

    debug!("Hardware interrupts initialized!");

    let page_range = {
        let start = VirtAddr::new(ACPI_START_ADDRESS as u64);
        let end = start + ACPI_MEMORY_SIZE as u64 - 1u64;
        let start_page = Page::containing_address(start);
        let end_page = Page::containing_address(end);
        Page::range_inclusive(start_page, end_page)
    };
    for page in page_range {
        ACPI_PAGES.push(page);
    }

    debug!("acpi init",);
    let acpi = unsafe {
        acpi::AcpiTables::from_rsdp(AcpiHandler {}, rsdp).expect("reading acpi did not succed!")
    };

    let platform_info = acpi.platform_info().unwrap();
    let processor_info = platform_info.processor_info.unwrap();
    let processors = processor_info.application_processors;

    unsafe { xapic().ipi_startup(ApicId::XApic(0), 0x08) };
    debug!("boot processor: {:?}", processor_info.boot_processor);
    for proc in processors.iter() {
        debug!("processor : {proc:?}");
    }
    debug!("acpi: {:?}", acpi.platform_info());
}

fn map_memory_for_io_apic() {
    let mut mapper = MAPPER.get().expect("memory was not yet initialized").lock();
    let mut frame_allocator = StaticFrameAllocator {};

    let io_apic_address = VirtAddr::new(IOAPIC_ADDR as u64);
    let page: x86_64::structures::paging::Page = Page::containing_address(io_apic_address);
    let frame = PhysFrame::containing_address(PhysAddr::new(IOAPIC_ADDR as u64));
    let flags = PageTableFlags::PRESENT | PageTableFlags::NO_CACHE | PageTableFlags::WRITABLE;

    unsafe {
        mapper
            .map_to(page, frame, flags, &mut frame_allocator)
            .expect("mapping memory for io apic did not succeed")
            .flush()
    };
}

use x86_64::structures::idt::{InterruptDescriptorTable, PageFaultErrorCode};

use crate::{
    allocator::{HEAP_SIZE, HEAP_START},
    hlt_loop,
    memory::{MAPPER, StaticFrameAllocator},
};
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    xapic().eoi();
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    log::error!("EXCEPTION: PAGE FAULT");
    log::error!("Accessed Address: {:?}", Cr2::read());
    log::error!("Error Code: {:?}", error_code);
    log::error!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    xapic().eoi();
}
// not working due to recent regression in nightly, have to try later
// extern "x86-interrupt" fn double_fault_handler(
//     stack_frame: InterruptStackFrame,
//     _error_code: u64,
// ) -> ! {
//     panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
//     loop {}
// }

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
