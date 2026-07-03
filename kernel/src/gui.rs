//! Graphical dashboard console for the TXPOS kernel.

use txpos_bootloader::FrameBufferInfo;

/// Renders a beautiful system dashboard in the kernel graphical framebuffer.
pub fn render_dashboard(fb: &FrameBufferInfo) {
    let fb_base = fb.base;
    let stride = fb.stride;
    let width = fb.width;
    let height = fb.height;

    // Helper functions for drawing
    let draw_pixel = |x: u32, y: u32, color: u32| {
        if x < width && y < height {
            let offset = (y as u64 * stride as u64 + x as u64) * 4;
            unsafe {
                let ptr = (fb_base + offset) as *mut u32;
                ptr.write_volatile(color);
            }
        }
    };

    let draw_rect = |x: u32, y: u32, w: u32, h: u32, color: u32| {
        for py in y..(y + h) {
            for px in x..(x + w) {
                draw_pixel(px, py, color);
            }
        }
    };

    // 1. Draw a dark gradient background (Deep violet to dark navy)
    for y in 0..height {
        let r = (10 + (y * 10 / height)) as u8;
        let g = (12 + (y * 8 / height)) as u8;
        let b = (24 + (y * 16 / height)) as u8;
        let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        for x in 0..width {
            draw_pixel(x, y, color);
        }
    }

    // 2. Draw Top Status Panel (Top Bar)
    let bar_h = 40;
    draw_rect(0, 0, width, bar_h, 0x001B_1B29); // Dark grey-blue bar
    draw_rect(0, bar_h - 1, width, 1, 0x00FF_AA00); // Gold separation line

    // 3. Draw Side Panel (Left Sidebar) for security statuses
    let side_w = 200;
    draw_rect(0, bar_h, side_w, height - bar_h, 0x0015_1520); // Darker sidebar background
    draw_rect(side_w - 1, bar_h, 1, height - bar_h, 0x0033_3344); // Sidebar border

    // Sidebar status boxes
    // TXShield Active (Neon Green)
    draw_rect(20, bar_h + 30, 160, 30, 0x000F_380F);
    draw_rect(20, bar_h + 30, 160, 30, 0x0000_FF66); // Border/Fill indicator
    draw_rect(22, bar_h + 32, 156, 26, 0x000F_380F);

    // TXSentinel Active (Vibrant Cyan)
    draw_rect(20, bar_h + 80, 160, 30, 0x0005_2D3A);
    draw_rect(20, bar_h + 80, 160, 30, 0x0000_E5FF);
    draw_rect(22, bar_h + 82, 156, 26, 0x0005_2D3A);

    // TXVault Mounted (Gold)
    draw_rect(20, bar_h + 130, 160, 30, 0x003A_2705);
    draw_rect(20, bar_h + 130, 160, 30, 0x00FF_AA00);
    draw_rect(22, bar_h + 132, 156, 26, 0x003A_2705);

    // TXFirewall Shielded (Vibrant Orange)
    draw_rect(20, bar_h + 180, 160, 30, 0x003A_1405);
    draw_rect(20, bar_h + 180, 160, 30, 0x00FF_5500);
    draw_rect(22, bar_h + 182, 156, 26, 0x003A_1405);

    // 4. Central Dashboard Window (System Logs / Shell Console)
    let win_x = side_w + 30;
    let win_y = bar_h + 30;
    let win_w = width - side_w - 60;
    let win_h = height - bar_h - 60;

    // Window Frame/Shadow
    draw_rect(win_x, win_y, win_w, win_h, 0x000D_0E15); // Black glass container
    draw_rect(win_x, win_y, win_w, 25, 0x0022_2530); // Window Title Bar
    draw_rect(win_x, win_y, win_w, win_h, 0x00FF_AA00); // Window Border Outline
    draw_rect(win_x + 1, win_y + 1, win_w - 2, win_h - 2, 0x000D_0E15);
    draw_rect(win_x, win_y, win_w, 25, 0x0022_2530); // Re-draw Title Bar

    // Window close/minimize/maximize buttons (OS style)
    draw_rect(win_x + 10, win_y + 8, 10, 10, 0x00FF_3B30); // Red
    draw_rect(win_x + 25, win_y + 8, 10, 10, 0x00FF_CC00); // Yellow
    draw_rect(win_x + 40, win_y + 8, 10, 10, 0x0034_C759); // Green

    // 5. Draw a static mouse cursor (Vibrant Gold Arrow)
    let mx = width / 2 + 50;
    let my = height / 2 + 50;
    for i in 0..12 {
        draw_rect(mx + i, my + i, 2, 2, 0x00FF_AA00);
        draw_rect(mx, my + i, 2, 2, 0x00FF_AA00);
        draw_rect(mx + i, my, 2, 2, 0x00FF_AA00);
    }
}
