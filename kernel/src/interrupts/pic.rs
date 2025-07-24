#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
use lazy_static::*;
use x86_64::structures::idt::InterruptStackFrame;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);

        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}

use pic8259::ChainedPics;
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub fn disable_pic() {
    unsafe { PICS.lock().disable() };
    // unsafe {
    //     // the naming convention for slave and master PIC is really weird... guy that was naming
    //     // this was really fucked up
    //
    //     // Initialization Control Word 1
    //     port::PortWrite::write_to_port(0x20, 0x11_u8);
    //     port::PortWrite::write_to_port(0xA0, 0x11_u8);
    //
    //     // Vector offset
    //     port::PortWrite::write_to_port(0x21, 0x20_u8); // Master PIC vector offset
    //     port::PortWrite::write_to_port(0xA1, 0x28_u8); // Slave PIC vector offset
    //
    //     // Tell Master PIC there is a slave PIC at IRQ2 (0000 0100)
    //     port::PortWrite::write_to_port(0x21, 0x04_u8);
    //     port::PortWrite::write_to_port(0xA1, 0x02_u8);
    //
    //     // Set environment info
    //     port::PortWrite::write_to_port(0x21, 0x01_u8);
    //     port::PortWrite::write_to_port(0xA1, 0x01_u8);
    //
    //     // Mask all interrupts
    //     port::PortWrite::write_to_port(0x21, 0xFF_u8);
    //     port::PortWrite::write_to_port(0xA1, 0xFF_u8);
    // }
    log::debug!("pic was disabled!");
}
pub fn init() {
    // x86_64::ap
    log::debug!("IDT initialized!");

    IDT.load();

    unsafe { PICS.lock().initialize() };

    x86_64::instructions::interrupts::enable();

    log::debug!("Hardware interrupts initialized!");
}

use x86_64::{
    instructions::port,
    structures::idt::{InterruptDescriptorTable, PageFaultErrorCode},
};

use crate::hlt_loop;
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
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

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
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
