use core::ptr::write_volatile;

use acpi::PhysicalMapping;
use log::debug;
use spin::Mutex;
use x86::apic::{ApicControl, ApicId};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
};
const APIC_LEN: usize = 1024;

fn io_apic() -> x86::apic::ioapic::IoApic {
    unsafe { x86::apic::ioapic::IoApic::new(IOAPIC_ADDR) }
}
fn xapic() -> x86::apic::xapic::XAPIC {
    let apic_address = VirtAddr::new(LOCAL_APIC_ADDR as u64);
    let pointer = apic_address.as_mut_ptr();

    let slice: &'static mut [u32] = unsafe { core::slice::from_raw_parts_mut(pointer, APIC_LEN) };

    x86::apic::xapic::XAPIC::new(slice)
}

fn init_xapic(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {

    let apic_address = VirtAddr::new(LOCAL_APIC_ADDR as u64);
    let page = Page::containing_address(apic_address);
    let frame = PhysFrame::containing_address(PhysAddr::new(LOCAL_APIC_ADDR));

    let flags = PageTableFlags::PRESENT | PageTableFlags::NO_CACHE | PageTableFlags::WRITABLE;
    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .expect("mapping memory for apic did not succeed")
            .flush()
    };

    let mut xapic = xapic();
    debug!("xapic id {:?}", xapic);
    debug!("xapic id {:?}", xapic.id());
    // xapic.attach();

    debug!("xapic bsp {:?}", xapic.bsp());
    // xapic.
    xapic.attach();

    // unsafe { xapic.ipi_init(ApicId::XApic(0)) };
    // xapic.tsc_enable(1);
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

        idt[KEYBOARD_IRQ + IRQ_BASE].set_handler_fn(keyboard_interrupt_handler);
        idt[TIMER_IRQ + IRQ_BASE].set_handler_fn(timer_interrupt_handler);

        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}

pub const PIC_1_OFFSET: u8 = 32;

pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    init_xapic(mapper, frame_allocator);

    map_memory_for_io_apic(mapper, frame_allocator);

    IDT.load();

    log::debug!("IDT initialized!");
    let mut io_apic = io_apic();

    io_apic.enable(KEYBOARD_IRQ, 0);
    io_apic.enable(TIMER_IRQ, 0);

    x86_64::instructions::interrupts::enable();

    log::debug!("Hardware interrupts initialized!");
}

fn map_memory_for_io_apic(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let io_apic_address = VirtAddr::new(IOAPIC_ADDR as u64);
    let page = Page::containing_address(io_apic_address);
    let frame = PhysFrame::containing_address(PhysAddr::new(IOAPIC_ADDR as u64));
    let flags = PageTableFlags::PRESENT | PageTableFlags::NO_CACHE | PageTableFlags::WRITABLE;

    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .expect("mapping memory for io apic did not succeed")
            .flush()
    };
}

use x86_64::{
    instructions::port,
    structures::idt::{InterruptDescriptorTable, PageFaultErrorCode},
};

use crate::{hlt_loop, interrupts, memory::BootInfoFrameAllocator};
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    debug!("Timer!");
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
const LAPIC_BASE: u64 = 0xFEE00000;
const EOI_OFFSET: usize = 0xB0;

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    log::debug!("keyboard 1");
    crate::task::keyboard::add_scancode(scancode);

    log::debug!("keyboard 2");

    // xapic().eoi();
    unsafe {
        interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(KEYBOARD_IRQ)
    };

    let eoi_reg = (LAPIC_BASE + 176 as u64) as *mut u32;
    unsafe { write_volatile(eoi_reg, 0) };
    //
    log::debug!("keyboard 3");

    // io_apic().enable(KEYBOARD_IRQ, 0);
}
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
