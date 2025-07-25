#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use acpi::rsdp;
use bootloader_api::{
    BootInfo,
    config::{BootloaderConfig, Mapping},
    entry_point,
    info::MemoryRegions,
};
use conquer_once::spin::OnceCell;
use log::debug;
use spin::mutex::Mutex;

use crate::task::{StaticTask, keyboard::ScancodeStream};

// add a `config` argument to the `entry_point` macro call
pub mod allocator;
pub mod framebuffer;
pub mod graphics;
pub mod interrupts;
pub mod logger;
pub mod memory;
pub mod panic;
pub mod qemu;
pub mod serial;
pub mod task;

pub fn cpuid() {
    debug!("cpuid {:?}", x86::cpuid::CpuId::new());
}

pub fn init_kernel(boot_info: &'static mut bootloader_api::BootInfo) {
    logger::init_logger(log::LevelFilter::Debug);
    let physical_memory_offset = boot_info
        .physical_memory_offset
        .as_ref()
        .expect("physical memory offset was not provided by bootloader!")
        .clone();

    let memory_map = MemoryRegions::from(boot_info.memory_regions.as_mut());

    unsafe { memory::init_mem(physical_memory_offset, memory_map) };

    allocator::init_heap().expect("heap initialization failed");

    let framebuffer = boot_info
        .framebuffer
        .as_mut()
        .expect("No framebuffer provided");
    // framebuffer::init_frame_buffer(framebuffer);
    graphics::RENDERER
        .get_or_init(move || Mutex::new(graphics::FrameBufferRenderer::new(framebuffer)));

    interrupts::init(boot_info.rsdp_addr.take().unwrap() as usize);

    debug!("Initialization fished successfully!");
}
pub fn start_task_executor_loop() -> ! {
    debug!("start_task_executor_loop");
    let mut executor = task::executor::Executor::new();
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(task::keyboard::print_keypresses()));
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(logger::handel_log_que()));
    executor.run();
}
pub fn hlt_loop() -> ! {
    log::warn!("entering hlt loop!");
    loop {
        x86_64::instructions::hlt();
    }
}
