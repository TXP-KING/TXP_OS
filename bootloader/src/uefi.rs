#![no_std]
#![no_main]
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
//! Minimal TXPOS UEFI boot application with GOP GUI and Kernel Loading support.

use core::ffi::c_void;
use core::hint::spin_loop;
use core::panic::PanicInfo;

use txpos_bootloader::{BootInfo, FrameBufferInfo};

type EfiHandle = *mut c_void;
type EfiStatus = usize;

#[allow(dead_code)]
const EFI_SUCCESS: EfiStatus = 0;
const EFI_LOAD_ERROR: EfiStatus = 0x8000_0000_0000_0001;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EfiGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xd0, 0xe1, 0x7a, 0x45, 0xae],
};

const EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x0967e590,
    data2: 0x0a54,
    data3: 0x11d2,
    data4: [0x8e, 0x4f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

#[repr(C)]
struct EfiTableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
struct EfiSimpleTextOutputProtocol {
    reset: usize,
    output_string: unsafe extern "efiapi" fn(
        this: *mut EfiSimpleTextOutputProtocol,
        string: *const u16,
    ) -> EfiStatus,
    test_string: usize,
    query_mode: usize,
    set_mode: usize,
    set_attribute: usize,
    clear_screen: usize,
    set_cursor_position: usize,
    enable_cursor: usize,
    mode: *mut c_void,
}

#[repr(C)]
struct EfiSystemTable {
    header: EfiTableHeader,
    firmware_vendor: *mut u16,
    firmware_revision: u32,
    console_in_handle: EfiHandle,
    console_in: *mut c_void,
    console_out_handle: EfiHandle,
    console_out: *mut EfiSimpleTextOutputProtocol,
    standard_error_handle: EfiHandle,
    standard_error: *mut EfiSimpleTextOutputProtocol,
    runtime_services: *mut c_void,
    boot_services: *mut EfiBootServices,
    number_of_table_entries: usize,
    configuration_table: *mut c_void,
}

#[repr(C)]
struct EfiBootServices {
    hdr: EfiTableHeader,
    raise_tpl: usize,
    restore_tpl: usize,
    allocate_pages: unsafe extern "efiapi" fn(
        allocate_type: u32,
        memory_type: u32,
        pages: usize,
        memory: *mut u64,
    ) -> EfiStatus,
    free_pages: usize,
    get_memory_map: usize,
    allocate_pool: unsafe extern "efiapi" fn(
        pool_type: u32,
        size: usize,
        buffer: *mut *mut c_void,
    ) -> EfiStatus,
    free_pool: usize,
    create_event: usize,
    set_timer: usize,
    wait_for_event: usize,
    signal_event: usize,
    close_event: usize,
    check_event: usize,
    install_protocol_interface: usize,
    reinstall_protocol_interface: usize,
    uninstall_protocol_interface: usize,
    handle_protocol: usize,
    reserved: usize,
    register_protocol_notify: usize,
    locate_handle: usize,
    locate_device_path: usize,
    install_configuration_table: usize,
    load_image: usize,
    start_image: usize,
    exit: usize,
    unload_image: usize,
    exit_boot_services: usize,
    get_next_monotonic_count: usize,
    stall: unsafe extern "efiapi" fn(microseconds: usize) -> EfiStatus,
    set_watchdog_timer: usize,
    connect_controller: usize,
    disconnect_controller: usize,
    open_protocol: usize,
    close_protocol: usize,
    open_protocol_information: usize,
    protocols_per_handle: usize,
    locate_handle_buffer: usize,
    locate_protocol: unsafe extern "efiapi" fn(
        protocol: *const EfiGuid,
        registration: *mut c_void,
        interface: *mut *mut c_void,
    ) -> EfiStatus,
}

#[repr(C)]
struct EfiGraphicsOutputProtocol {
    query_mode: usize,
    set_mode: unsafe extern "efiapi" fn(
        this: *mut EfiGraphicsOutputProtocol,
        mode_number: u32,
    ) -> EfiStatus,
    blt: usize,
    mode: *mut EfiGraphicsOutputProtocolMode,
}

#[repr(C)]
struct EfiGraphicsOutputProtocolMode {
    max_mode: u32,
    mode: u32,
    info: *mut EfiGraphicsOutputModeInformation,
    size_of_info: usize,
    frame_buffer_base: u64,
    frame_buffer_size: usize,
}

#[repr(C)]
struct EfiGraphicsOutputModeInformation {
    version: u32,
    horizontal_resolution: u32,
    vertical_resolution: u32,
    pixel_format: u32,
    pixel_information: [u32; 4],
    pixels_per_scan_line: u32,
}

#[repr(C)]
struct EfiSimpleFileSystemProtocol {
    revision: u64,
    open_volume: unsafe extern "efiapi" fn(
        this: *mut EfiSimpleFileSystemProtocol,
        root: *mut *mut EfiFileProtocol,
    ) -> EfiStatus,
}

#[repr(C)]
struct EfiFileProtocol {
    revision: u64,
    open: unsafe extern "efiapi" fn(
        this: *mut EfiFileProtocol,
        new_handle: *mut *mut EfiFileProtocol,
        file_name: *const u16,
        open_mode: u64,
        attributes: u64,
    ) -> EfiStatus,
    close: unsafe extern "efiapi" fn(
        this: *mut EfiFileProtocol,
    ) -> EfiStatus,
    delete: usize,
    read: unsafe extern "efiapi" fn(
        this: *mut EfiFileProtocol,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> EfiStatus,
    write: usize,
    get_position: usize,
    set_position: unsafe extern "efiapi" fn(
        this: *mut EfiFileProtocol,
        position: u64,
    ) -> EfiStatus,
    get_info: usize,
    set_info: usize,
    flush: usize,
}

struct Console {
    output: *mut EfiSimpleTextOutputProtocol,
}

impl Console {
    const fn new(output: *mut EfiSimpleTextOutputProtocol) -> Self {
        Self { output }
    }

    fn write_line(&mut self, text: &str) -> EfiStatus {
        let mut buffer = [0u16; 160];
        let mut index = 0;

        for byte in text.bytes() {
            if index + 3 >= buffer.len() {
                break;
            }

            buffer[index] = if byte.is_ascii() {
                byte as u16
            } else {
                b'?' as u16
            };
            index += 1;
        }

        buffer[index] = b'\r' as u16;
        buffer[index + 1] = b'\n' as u16;

        self.write_utf16_z(&buffer)
    }

    fn write_utf16_z(&mut self, text: &[u16]) -> EfiStatus {
        // SAFETY: `self.output` comes from the firmware system table after a
        // null check in `efi_main`. The buffer is stack-allocated,
        // null-terminated, and lives for the whole firmware call.
        unsafe { ((*self.output).output_string)(self.output, text.as_ptr()) }
    }
}

unsafe fn draw_pixel(fb_base: u64, stride: u32, x: u32, y: u32, color: u32) {
    let offset = (y as u64 * stride as u64 + x as u64) * 4;
    unsafe {
        let ptr = (fb_base + offset) as *mut u32;
        ptr.write_volatile(color);
    }
}

unsafe fn draw_rect(fb_base: u64, stride: u32, x: u32, y: u32, w: u32, h: u32, color: u32) {
    for py in y..(y + h) {
        for px in x..(x + w) {
            unsafe {
                draw_pixel(fb_base, stride, px, py, color);
            }
        }
    }
}

unsafe fn draw_gui_bootloader(
    boot_services: *mut EfiBootServices,
    fb_base: u64,
    width: u32,
    height: u32,
    stride: u32,
) {
    // 1. Draw smooth gradient background (Dark steel to charcoal black)
    for y in 0..height {
        let r = (15 + (y * 15 / height)) as u8;
        let g = (20 + (y * 20 / height)) as u8;
        let b = (30 + (y * 25 / height)) as u8;
        let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        for x in 0..width {
            unsafe {
                draw_pixel(fb_base, stride, x, y, color);
            }
        }
    }

    // Centered panel dimensions
    let panel_w = 460;
    let panel_h = 300;
    let panel_x = (width - panel_w) / 2;
    let panel_y = (height - panel_h) / 2;

    // 2. Draw card background with gold border
    for y in panel_y..(panel_y + panel_h) {
        for x in panel_x..(panel_x + panel_w) {
            let is_border = x == panel_x || x == panel_x + panel_w - 1 || y == panel_y || y == panel_y + panel_h - 1;
            let color = if is_border {
                0x00FF_AA00 // Gold borders
            } else {
                0x0012_161F // Deep slate blue card fill
            };
            unsafe {
                draw_pixel(fb_base, stride, x, y, color);
            }
        }
    }

    // 3. Draw Stylized Geometric "TX" logo
    let cx = panel_x + panel_w / 2;
    let cy = panel_y + 80;

    // Draw 'T' (Gold)
    unsafe {
        draw_rect(fb_base, stride, cx - 60, cy - 40, 48, 12, 0x00FF_AA00);
        draw_rect(fb_base, stride, cx - 42, cy - 28, 12, 48, 0x00FF_AA00);
    }

    // Draw 'X' (Vibrant Cyan)
    for i in 0..48 {
        unsafe {
            draw_rect(fb_base, stride, cx + 10 + i, cy - 40 + i, 8, 8, 0x0000_E5FF);
            draw_rect(fb_base, stride, cx + 58 - i, cy - 40 + i, 8, 8, 0x0000_E5FF);
        }
    }

    // 4. Progress bar dimensions
    let pb_x = panel_x + 50;
    let pb_y = panel_y + 180;
    let pb_w = 360;
    let pb_h = 16;

    // Draw outer progress bar frame
    for y in pb_y..(pb_y + pb_h) {
        for x in pb_x..(pb_x + pb_w) {
            let is_border = x == pb_x || x == pb_x + pb_w - 1 || y == pb_y || y == pb_y + pb_h - 1;
            if is_border {
                unsafe {
                    draw_pixel(fb_base, stride, x, y, 0x004F_5B66);
                }
            }
        }
    }

    // 5. Run animated loading steps (simulated Secure Boot checks)
    let steps = 4;
    for step in 1..=steps {
        // Draw progress fill for this step
        let fill_w = (pb_w - 4) * step / steps;
        unsafe {
            draw_rect(fb_base, stride, pb_x + 2, pb_y + 2, fill_w, pb_h - 4, 0x0000_FF66); // Neon green fill
        }

        // Draw checkmark indicator next to status list
        let check_x = panel_x + 60;
        let check_y = panel_y + 215 + (step - 1) * 16;
        
        // Draw a green square representing the checkmark / status dot
        unsafe {
            draw_rect(fb_base, stride, check_x, check_y, 8, 8, 0x0000_FF66);
        }

        // Stall for 250,000 microseconds (250ms) to show progress
        unsafe {
            ((*boot_services).stall)(250_000);
        }
    }
}

/// UEFI firmware entry point for the TXPOS bootloader.
#[unsafe(no_mangle)]
extern "efiapi" fn efi_main(
    _image_handle: EfiHandle,
    system_table: *mut EfiSystemTable,
) -> EfiStatus {
    if system_table.is_null() {
        return EFI_LOAD_ERROR;
    }

    // SAFETY: UEFI passes a valid system table pointer to `efi_main` while the
    // application is executing. The pointer was checked for null above.
    let console_out = unsafe { (*system_table).console_out };
    if console_out.is_null() {
        return EFI_LOAD_ERROR;
    }

    let mut console = Console::new(console_out);
    let _ = console.write_line("TXPOS bootloader started");
    let _ = console.write_line("TXPOS milestone 1 UEFI entry reached");
    
    // Locate GOP
    let boot_services = unsafe { (*system_table).boot_services };
    let mut gop: *mut EfiGraphicsOutputProtocol = core::ptr::null_mut();
    
    let gop_status = if boot_services.is_null() {
        EFI_LOAD_ERROR
    } else {
        unsafe {
            ((*boot_services).locate_protocol)(
                &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
                core::ptr::null_mut(),
                &mut gop as *mut *mut EfiGraphicsOutputProtocol as *mut *mut c_void,
            )
        }
    };

    if gop_status == 0 && !gop.is_null() {
        let mode = unsafe { (*gop).mode };
        if !mode.is_null() && !unsafe { (*mode).info }.is_null() {
            let fb_base = unsafe { (*mode).frame_buffer_base };
            let width = unsafe { (*(*mode).info).horizontal_resolution };
            let height = unsafe { (*(*mode).info).vertical_resolution };
            let stride = unsafe { (*(*mode).info).pixels_per_scan_line };
            
            // Render the graphical boot screen
            unsafe {
                draw_gui_bootloader(boot_services, fb_base, width, height, stride);
            }
        }
    } else {
        let _ = console.write_line("GOP not available, falling back to text mode");
    }

    let _ = console.write_line("Locating SimpleFileSystem...");
    
    let mut sfs: *mut EfiSimpleFileSystemProtocol = core::ptr::null_mut();
    let sfs_status = if boot_services.is_null() {
        EFI_LOAD_ERROR
    } else {
        unsafe {
            ((*boot_services).locate_protocol)(
                &EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
                core::ptr::null_mut(),
                &mut sfs as *mut *mut EfiSimpleFileSystemProtocol as *mut *mut c_void,
            )
        }
    };

    if sfs_status != 0 || sfs.is_null() {
        let _ = console.write_line("Failed to locate SimpleFileSystemProtocol");
        loop { spin_loop(); }
    }

    let mut root: *mut EfiFileProtocol = core::ptr::null_mut();
    let open_vol_status = unsafe { ((*sfs).open_volume)(sfs, &mut root) };
    if open_vol_status != 0 || root.is_null() {
        let _ = console.write_line("Failed to open root volume");
        loop { spin_loop(); }
    }

    let mut file: *mut EfiFileProtocol = core::ptr::null_mut();
    let path = [
        '\\' as u16, 'T' as u16, 'X' as u16, 'P' as u16, 'O' as u16, 'S' as u16, '\\' as u16,
        'K' as u16, 'E' as u16, 'R' as u16, 'N' as u16, 'E' as u16, 'L' as u16,
        '.' as u16, 'B' as u16, 'I' as u16, 'N' as u16, 0u16
    ];
    let open_file_status = unsafe { ((*root).open)(root, &mut file, path.as_ptr(), 1, 0) };
    if open_file_status != 0 || file.is_null() {
        let _ = console.write_line("Failed to open \\TXPOS\\KERNEL.BIN");
        loop { spin_loop(); }
    }

    // Allocate pool memory for raw file
    let mut temp_buffer: *mut u8 = core::ptr::null_mut();
    let alloc_pool_status = unsafe {
        ((*boot_services).allocate_pool)(
            2, // EfiLoaderData
            524_288,
            &mut temp_buffer as *mut *mut u8 as *mut *mut c_void,
        )
    };
    if alloc_pool_status != 0 || temp_buffer.is_null() {
        let _ = console.write_line("Failed to allocate memory pool for kernel file");
        loop { spin_loop(); }
    }

    let mut size: usize = 524_288;
    let read_status = unsafe { ((*file).read)(file, &mut size, temp_buffer as *mut c_void) };
    if read_status != 0 {
        let _ = console.write_line("Failed to read kernel file");
        loop { spin_loop(); }
    }

    // Verify ELF signature
    let signature = unsafe { core::slice::from_raw_parts(temp_buffer, 4) };
    if signature != b"\x7fELF" {
        let _ = console.write_line("Kernel is not a valid ELF binary");
        loop { spin_loop(); }
    }

    // Read ELF entry point and headers
    let entry_point_addr = unsafe { *(temp_buffer.add(24) as *const u64) };
    let phoff = unsafe { *(temp_buffer.add(32) as *const u64) };
    let phentsize = unsafe { *(temp_buffer.add(54) as *const u16) } as usize;
    let phnum = unsafe { *(temp_buffer.add(56) as *const u16) } as usize;

    // Allocate 128 pages (512 KB) of page-aligned memory for the loaded kernel segments
    let mut kernel_pages_addr: u64 = 0;
    let alloc_pages_status = unsafe {
        ((*boot_services).allocate_pages)(
            0, // AllocateAnyPages
            2, // EfiLoaderData
            128,
            &mut kernel_pages_addr,
        )
    };
    if alloc_pages_status != 0 || kernel_pages_addr == 0 {
        let _ = console.write_line("Failed to allocate physical pages for kernel mapping");
        loop { spin_loop(); }
    }

    // Zero out memory region first
    let kernel_mem = kernel_pages_addr as *mut u8;
    for i in 0..524_288 {
        unsafe {
            kernel_mem.add(i).write(0);
        }
    }

    // Load ELF PT_LOAD segments relative to kernel_pages_addr
    for i in 0..phnum {
        let ph_ptr = unsafe { temp_buffer.add(phoff as usize + i * phentsize) };
        let p_type = unsafe { *(ph_ptr as *const u32) };
        if p_type == 1 { // PT_LOAD
            let p_offset = unsafe { *(ph_ptr.add(8) as *const u64) } as usize;
            let p_vaddr = unsafe { *(ph_ptr.add(16) as *const u64) } as usize;
            let p_filesz = unsafe { *(ph_ptr.add(32) as *const u64) } as usize;
            let p_memsz = unsafe { *(ph_ptr.add(40) as *const u64) } as usize;

            let dest = (kernel_pages_addr as usize + p_vaddr) as *mut u8;
            let src = unsafe { temp_buffer.add(p_offset) };

            for j in 0..p_filesz {
                unsafe {
                    dest.add(j).write(src.add(j).read());
                }
            }

            for j in p_filesz..p_memsz {
                unsafe {
                    dest.add(j).write(0);
                }
            }
        }
    }

    // Close open handles
    let _ = unsafe { ((*file).close)(file) };
    let _ = unsafe { ((*root).close)(root) };

    // Setup BootInfo
    let mut boot_info = BootInfo {
        physical_memory_offset: 0,
        memory_map_start: 0x5000,
        memory_map_len: 4096,
        rsdp_addr: None,
        framebuffer: None,
    };

    if gop_status == 0 && !gop.is_null() {
        let mode = unsafe { (*gop).mode };
        if !mode.is_null() && !unsafe { (*mode).info }.is_null() {
            let fb_base = unsafe { (*mode).frame_buffer_base };
            let width = unsafe { (*(*mode).info).horizontal_resolution };
            let height = unsafe { (*(*mode).info).vertical_resolution };
            let stride = unsafe { (*(*mode).info).pixels_per_scan_line };
            
            boot_info.framebuffer = Some(FrameBufferInfo {
                base: fb_base,
                length: (stride * height * 4) as u64,
                width,
                height,
                stride,
            });
        }
    }

    // Construct kernel entry function pointer and call it
    let entry_point: extern "C" fn(boot_info: &'static BootInfo) -> ! = unsafe {
        core::mem::transmute(kernel_pages_addr + entry_point_addr)
    };

    let _ = console.write_line("Jumping to Kernel Entry point...");

    // Jump!
    entry_point(unsafe { core::mem::transmute(&boot_info) });
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        spin_loop();
    }
}
