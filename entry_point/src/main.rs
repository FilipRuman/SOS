#![no_std]
#![no_main]

extern crate alloc;
use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();

    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(entry_point, config = &BOOTLOADER_CONFIG);
fn entry_point(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    kernel::init_kernel(boot_info);
    os::init_os();

    kernel::hlt_loop();
}
