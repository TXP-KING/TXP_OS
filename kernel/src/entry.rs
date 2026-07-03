#![no_std]
#![no_main]
#![allow(unsafe_code)]
//! Minimal bare-metal TXPOS kernel entry for milestone 1.
//!
//! The exported `_start` symbol is the future handoff point from the UEFI
//! bootloader. The only unsafe construct here is the required external symbol
//! export; the body stays in safe Rust.

use core::hint::spin_loop;
use core::panic::PanicInfo;

use txpos_bootloader::BootInfo;
use txpos_kernel::{Kernel, KernelConfig};
use txpos_memory::PAGE_SIZE;

mod gui;

/// Bare-metal kernel entry point.
#[unsafe(no_mangle)]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    let _kernel = Kernel::<4, 4>::init(KernelConfig {
        memory_start: 0x1000,
        memory_len: PAGE_SIZE * 16,
        kernel_digest: [0; 32],
    });

    if let Some(ref fb) = boot_info.framebuffer {
        gui::render_dashboard(fb);
    }

    loop {
        spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        spin_loop();
    }
}
