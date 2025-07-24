use log::{debug, *};
use x86::apic::ApicControl;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
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

fn init_xapic(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let apic_address = VirtAddr::new(LOCAL_APIC_ADDR);
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

pub const PIC_1_OFFSET: u8 = 32;

pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    init_xapic(mapper, frame_allocator);

    map_memory_for_io_apic(mapper, frame_allocator);

    IDT.load();

    debug!("IDT initialized!");
    setup_xapic_timer();

    let mut io_apic = io_apic();
    io_apic.enable(KEYBOARD_IRQ, 0);

    x86_64::instructions::interrupts::enable();

    debug!("Hardware interrupts initialized!");

    debug!("cpuid {:?}", x86::cpuid::CpuId::new());
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

use x86_64::structures::idt::{InterruptDescriptorTable, PageFaultErrorCode};

use crate::hlt_loop;
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
