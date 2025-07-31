#[repr(C, align(8))]
pub struct TrampolineData {
    pub ap_stack_ptr: u64,               // Top of AP stack
    pub ap_entry_point_address: u64,     // Function pointer to `ap_main`
    pub level_4_table_phys_address: u64, // PML4 physical address
    pub gdt_start_address: u64,
    pub gdt_size: u16,
}

pub const TRAMPOLINE_ADDR: usize = 0x8000;

use core::ptr;

use log::debug;
use x86_64::{
    VirtAddr,
    structures::paging::{Page, PageTableFlags},
};

use crate::{
    gdt,
    memory::{self, PER_AP_STACK_STACK_SIZE},
    threads::ap_entrypoint::ap_entrypoint,
};
const fn stack_ptr(cpu_id: u8) -> u64 {
    (memory::AP_STACK_MEMORY_START + memory::PER_AP_STACK_STACK_SIZE * cpu_id as usize) as u64
}

fn allocate_memory_for_stacks(cpu_count: usize) {
    unsafe {
        memory::map_memory(
            memory::AP_STACK_MEMORY_START,
            PER_AP_STACK_STACK_SIZE * cpu_count,
            PageTableFlags::NO_EXECUTE | PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
    };
}

static TRAMPOLINE_BIN: &[u8] = include_bytes!("trampoline.bin");
fn load_trampoline() {
    unsafe {
        memory::map_memory(
            0x8000,
            0x9000 - 0x8000 + 1,
            PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
        )
    };

    let trampoline_dst = TRAMPOLINE_ADDR as *mut u8;

    unsafe {
        ptr::copy_nonoverlapping(
            TRAMPOLINE_BIN.as_ptr(),
            trampoline_dst,
            TRAMPOLINE_BIN.len(),
        )
    };
}
pub fn init(ap_count: usize) {
    allocate_memory_for_stacks(ap_count);
    load_trampoline();
}
pub fn setup_trampoline_data(
    ap_id: u8,
    level_4_table_phys_address: u64,
    gdt_base_phys_address: u64,
    gdt_size: usize,
) {
    let data = TrampolineData {
        ap_stack_ptr: stack_ptr(ap_id),
        ap_entry_point_address: ap_entrypoint as u64,
        level_4_table_phys_address,
        gdt_start_address: gdt_base_phys_address,
        gdt_size: gdt_size as u16,
    };

    let dst = 0x9000 as *mut TrampolineData;
    unsafe { ptr::write(dst, data) };
}
