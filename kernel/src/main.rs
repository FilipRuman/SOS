#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    entry_point,
    info::MemoryRegions,
};
use log::debug;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();

    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

// add a `config` argument to the `entry_point` macro call
entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);
mod allocator;
mod framebuffer;
mod interrupts;
mod logger;
mod memory;
mod panic;
mod qemu;
mod serial;

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init_kernel(boot_info);

    hlt_loop();
}

fn init_kernel(boot_info: &'static mut bootloader_api::BootInfo) {

    let framebuffer = boot_info
        .framebuffer
        .as_mut()
        .expect("No framebuffer provided");
    framebuffer::init_frame_buffer(framebuffer);
    interrupts::init();

    let physical_memory_offset = boot_info
        .physical_memory_offset
        .as_ref()
        .expect("physical memory offset was not provided by bootloader!")
        .clone();

    let memory_map = MemoryRegions::from(boot_info.memory_regions.as_mut());
    let (mut mapper, mut frame_allocator) =
        unsafe { memory::init_mem(physical_memory_offset, memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    debug!("Initialization fished successfully!");
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
