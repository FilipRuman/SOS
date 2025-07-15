#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
// #![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

bootloader_api::entry_point!(kernel_main);
mod framebuffer;
mod logger;
mod serial;
use core::panic::PanicInfo;
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    framebuffer::init_frame_buffer(boot_info);

    log::debug!("test log!");

    loop {}
}
//  run on panic
#[cfg(not(test))] // new attribute
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}

    //   serial_println!("Error: {}\n", info);
    //   println!("{}", info);
    //    hlt_loop();
}

// our panic handler in test mode

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    //
    loop {}
    // use crate::quemu::{QemuExitCode, exit_qemu};
    //
    // serial_println!("[failed]\n");
    // serial_println!("Error: {}\n", info);
    // exit_qemu(QemuExitCode::Failed);
    // hlt_loop();
}
