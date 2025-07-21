#![no_std]

use alloc::vec;
extern crate alloc;
pub fn init_os() {
    let vec = vec!["os ", "is ", "initialized!"];
    log::debug!("{:?}", vec);
}
