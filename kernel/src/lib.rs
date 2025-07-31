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
pub mod gdt;

pub mod graphics;
pub mod interrupts;
pub mod logger;
pub mod memory;
pub mod panic;
pub mod qemu;
pub mod serial;
pub mod task;
pub mod threads;
pub mod time;

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

    let level_4_table_phys_address =
        unsafe { memory::init_mem(physical_memory_offset, memory_map) };

    allocator::init_heap().expect("heap initialization failed");

    let framebuffer = boot_info
        .framebuffer
        .as_mut()
        .expect("No framebuffer provided");
    // framebuffer::init_frame_buffer(framebuffer);
    graphics::RENDERER
        .get_or_init(move || Mutex::new(graphics::FrameBufferRenderer::new(framebuffer)));

    let (gdt_base_phys_address, gdt_size) = gdt::init();
    let rsdp = boot_info.rsdp_addr.take().unwrap();
    let cpu_count = interrupts::init(rsdp as usize);

    RSDP.get_or_init(|| rsdp);
    threads::init(
        cpu_count,
        level_4_table_phys_address,
        gdt_base_phys_address,
        gdt_size,
    );

    task::keyboard::ON_KEY_PRESSED_LISTENERS
        .lock()
        .push(on_key_debug_other_things);

    debug!("Initialization fished successfully!");
}

pub async fn test_debug_every_second() {
    loop {
        debug!("sec!");
        time::wait_ms(1000).await;
    }
}
static RSDP: OnceCell<u64> = OnceCell::uninit();
// used for easy triggering of debugs for all sorts of stuff
pub fn on_key_debug_other_things(_: &pc_keyboard::DecodedKey) {
    // interrupts::apic::init_acpi(*RSDP.get().unwrap() as usize);
    // debug!(
    //     "threads entrypoint test: {}",
    //     threads::ap_entrypoint::test.load(core::sync::atomic::Ordering::Relaxed)
    // );
}

pub fn start_task_executor_loop() -> ! {
    debug!("start_task_executor_loop");
    let mut executor = task::executor::Executor::new();
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(task::keyboard::print_keypresses()));
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(logger::handel_log_que()));
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(time::run_timer_loop()));
    task::executor::TASK_SPAWNER.spawn(StaticTask::new(test_debug_every_second()));

    executor.run();
}
pub fn hlt_loop() -> ! {
    log::warn!("entering hlt loop!");
    loop {
        x86_64::instructions::hlt();
    }
}
