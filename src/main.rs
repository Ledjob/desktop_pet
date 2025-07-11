use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CYSCREEN};
use std::{ffi::c_void, ptr::null_mut, thread, time::Duration, fs, io::Read};
use fontdue::{Font, FontSettings};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
            CreateDIBSection, DIB_RGB_COLORS, GetDC, SelectObject,  HDC, DeleteDC, DeleteObject, ReleaseDC,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, RegisterClassW, ShowWindow, UpdateLayeredWindow, ULW_ALPHA,
            CS_HREDRAW, CS_VREDRAW, SW_SHOW, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP,
            PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE, WM_QUIT,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, GetCursorPos,
        },
    },
};

mod utils;
mod scheduler;

fn to_wide(string: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(string).encode_wide().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            unsafe { windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}



fn main() {
    unsafe {
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        
        // Load all images
        let image_normal = image::open("assets/parrot.png").expect("parrot.png not found").to_rgba8();
        let image_low = image::open("assets/parrot_low.png").expect("parrot_low.png not found").to_rgba8();
        let image_fly1 = image::open("assets/parrot1.png").expect("parrot1.png not found").to_rgba8();
        let image_fly2 = image::open("assets/parrot2.png").expect("parrot2.png not found").to_rgba8();
        let image_fly3 = image::open("assets/parrot3.png").expect("parrot3.png not found").to_rgba8();
        let image_bubble = image::open("assets/bubble.png").expect("bubble.png not found").to_rgba8();
        
        
        let (img_w, img_h) = image_normal.dimensions();
        let (bubble_w, bubble_h) = image_bubble.dimensions();
        let image_normal_data = image_normal.as_flat_samples().samples;
        let image_low_data = image_low.as_flat_samples().samples;
        let image_fly1_data = image_fly1.as_flat_samples().samples;
        let image_fly2_data = image_fly2.as_flat_samples().samples;
        let image_fly3_data = image_fly3.as_flat_samples().samples;
        let image_bubble_data = image_bubble.as_flat_samples().samples;

        // Load messages from file
        let mut messages_file = fs::File::open("messages.txt").expect("messages.txt not found");
        let mut messages_content = String::new();
        messages_file.read_to_string(&mut messages_content).expect("Failed to read messages.txt");
        let messages: Vec<String> = messages_content.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        // Load font for Japanese text rendering
        let font_data = std::fs::read("C:/Users/EPSY GREEN/AppData/Local/Microsoft/Windows/Fonts/NotoSansCJKjp-Regular.otf")
            .expect("Font not found at C:/Windows/Fonts/NotoSansCJKjp-Regular.otf. Please check the path and that the font is installed.");
        let font = Font::from_bytes(font_data, FontSettings::default()).expect("Failed to load font");

        // Simple text rendering without font loading for now

        // Register window class
        let hinstance = GetModuleHandleW(None).unwrap().into();
        let class_name = to_wide("TransparentWindow");
        RegisterClassW(&WNDCLASSW {
            hInstance: hinstance,
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            lpfnWndProc: Some(window_proc),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        });

        // Scale down the parrot to 4x smaller (was 1.5x, now 4x)
        let scaled_w = img_w / utils::PARROT_SCALE;
        let scaled_h = img_h / utils::PARROT_SCALE;

        // Create transparent layered window - fullscreen
        let screen_width = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
        let ex_style = if utils::ALWAYS_ON_TOP {
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST
        } else {
            WS_EX_LAYERED | WS_EX_TOOLWINDOW
        };
        let hwnd = CreateWindowExW(
            ex_style,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(to_wide("Parrot Pet").as_ptr()),
            WS_POPUP,
            0,
            0,
            screen_width,
            screen_height,
            HWND(0),
            None,
            hinstance,
            Some(null_mut()),
        );

        let _ = ShowWindow(hwnd, SW_SHOW);

        // Create memory DC and DIB section
        let screen_dc: HDC = GetDC(HWND(0));
        let mem_dc: HDC = CreateCompatibleDC(screen_dc);

        // Scale bubble image (make it bigger than before)
        let scaled_bubble_w = bubble_w / utils::BUBBLE_SCALE; // Make bubble bigger (was /6, now /4)
        let scaled_bubble_h = bubble_h / utils::BUBBLE_SCALE;
        
        // Create a larger bitmap to hold both parrot and bubble
        let vertical_padding = 200; // Space above parrot for bubble
        let combined_width = scaled_w + scaled_bubble_w;
        let combined_height = (scaled_h + vertical_padding).max(scaled_bubble_h + vertical_padding); // Add extra height for bubble positioning
        
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: combined_width as i32,
                biHeight: -(combined_height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let h_bitmap = CreateDIBSection(
            mem_dc,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits_ptr as *mut *mut c_void,
            None,
            0,
        )
        .expect("CreateDIBSection failed");

        let old_bitmap = SelectObject(mem_dc, h_bitmap);

        // Create scaled bitmap data for all images
        let mut normal_bitmap_data: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut low_bitmap_data: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut fly1_bitmap_data: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut fly2_bitmap_data: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut fly3_bitmap_data: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        
        // Scale normal image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                normal_bitmap_data[dest_idx + 0] = image_normal_data[src_idx + 2]; // B
                normal_bitmap_data[dest_idx + 1] = image_normal_data[src_idx + 1]; // G
                normal_bitmap_data[dest_idx + 2] = image_normal_data[src_idx + 0]; // R
                normal_bitmap_data[dest_idx + 3] = image_normal_data[src_idx + 3]; // A
            }
        }
        
        // Scale low image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                low_bitmap_data[dest_idx + 0] = image_low_data[src_idx + 2]; // B
                low_bitmap_data[dest_idx + 1] = image_low_data[src_idx + 1]; // G
                low_bitmap_data[dest_idx + 2] = image_low_data[src_idx + 0]; // R
                low_bitmap_data[dest_idx + 3] = image_low_data[src_idx + 3]; // A
            }
        }
        
        // Scale fly1 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                fly1_bitmap_data[dest_idx + 0] = image_fly1_data[src_idx + 2]; // B
                fly1_bitmap_data[dest_idx + 1] = image_fly1_data[src_idx + 1]; // G
                fly1_bitmap_data[dest_idx + 2] = image_fly1_data[src_idx + 0]; // R
                fly1_bitmap_data[dest_idx + 3] = image_fly1_data[src_idx + 3]; // A
            }
        }
        
        // Scale fly2 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                fly2_bitmap_data[dest_idx + 0] = image_fly2_data[src_idx + 2]; // B
                fly2_bitmap_data[dest_idx + 1] = image_fly2_data[src_idx + 1]; // G
                fly2_bitmap_data[dest_idx + 2] = image_fly2_data[src_idx + 0]; // R
                fly2_bitmap_data[dest_idx + 3] = image_fly2_data[src_idx + 3]; // A
            }
        }
        
        // Scale fly3 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                fly3_bitmap_data[dest_idx + 0] = image_fly3_data[src_idx + 2]; // B
                fly3_bitmap_data[dest_idx + 1] = image_fly3_data[src_idx + 1]; // G
                fly3_bitmap_data[dest_idx + 2] = image_fly3_data[src_idx + 0]; // R
                fly3_bitmap_data[dest_idx + 3] = image_fly3_data[src_idx + 3]; // A
            }
        }

        // Create flipped versions for all images
        let mut flipped_normal_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut flipped_low_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut flipped_fly1_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut flipped_fly2_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        let mut flipped_fly3_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        
        // Flip normal image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_normal_bits[dest_idx + 0] = normal_bitmap_data[src_idx + 0]; // B
                flipped_normal_bits[dest_idx + 1] = normal_bitmap_data[src_idx + 1]; // G
                flipped_normal_bits[dest_idx + 2] = normal_bitmap_data[src_idx + 2]; // R
                flipped_normal_bits[dest_idx + 3] = normal_bitmap_data[src_idx + 3]; // A
            }
        }
        
        // Flip low image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_low_bits[dest_idx + 0] = low_bitmap_data[src_idx + 0]; // B
                flipped_low_bits[dest_idx + 1] = low_bitmap_data[src_idx + 1]; // G
                flipped_low_bits[dest_idx + 2] = low_bitmap_data[src_idx + 2]; // R
                flipped_low_bits[dest_idx + 3] = low_bitmap_data[src_idx + 3]; // A
            }
        }
        
        // Flip fly1 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_fly1_bits[dest_idx + 0] = fly1_bitmap_data[src_idx + 0]; // B
                flipped_fly1_bits[dest_idx + 1] = fly1_bitmap_data[src_idx + 1]; // G
                flipped_fly1_bits[dest_idx + 2] = fly1_bitmap_data[src_idx + 2]; // R
                flipped_fly1_bits[dest_idx + 3] = fly1_bitmap_data[src_idx + 3]; // A
            }
        }
        
        // Flip fly2 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_fly2_bits[dest_idx + 0] = fly2_bitmap_data[src_idx + 0]; // B
                flipped_fly2_bits[dest_idx + 1] = fly2_bitmap_data[src_idx + 1]; // G
                flipped_fly2_bits[dest_idx + 2] = fly2_bitmap_data[src_idx + 2]; // R
                flipped_fly2_bits[dest_idx + 3] = fly2_bitmap_data[src_idx + 3]; // A
            }
        }
        
        // Flip fly3 image
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_fly3_bits[dest_idx + 0] = fly3_bitmap_data[src_idx + 0]; // B
                flipped_fly3_bits[dest_idx + 1] = fly3_bitmap_data[src_idx + 1]; // G
                flipped_fly3_bits[dest_idx + 2] = fly3_bitmap_data[src_idx + 2]; // R
                flipped_fly3_bits[dest_idx + 3] = fly3_bitmap_data[src_idx + 3]; // A
            }
        }

        let mut bubble_bitmap_data: Vec<u8> = vec![0; (scaled_bubble_w * scaled_bubble_h * 4) as usize];
        
        // Scale bubble image
        for y in 0..scaled_bubble_h {
            for x in 0..scaled_bubble_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * bubble_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_bubble_w + x) as usize * 4;
                
                bubble_bitmap_data[dest_idx + 0] = image_bubble_data[src_idx + 2]; // B
                bubble_bitmap_data[dest_idx + 1] = image_bubble_data[src_idx + 1]; // G
                bubble_bitmap_data[dest_idx + 2] = image_bubble_data[src_idx + 0]; // R
                bubble_bitmap_data[dest_idx + 3] = image_bubble_data[src_idx + 3]; // A
            }
        }

        let mut pt_dst = POINT { x: 300, y: 300 };
        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: combined_width as i32,
            cy: combined_height as i32,
        };

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_ALPHA as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        // Physics and movement variables
        let mut velocity_y: f32 = 0.0;
        let mut velocity_x: f32 = 0.0;
        let gravity: f32 = 0.5;
        let mut position_y: f32 = pt_dst.y as f32;
        let mut position_x: f32 = pt_dst.x as f32;
        let mut last_drawn_y: i32 = pt_dst.y;
        let mut last_drawn_x: i32 = pt_dst.x;
        let mut msg = MSG::default();
        
        // Random movement variables with proper RNG
        let mut rng = utils::SimpleRng::new();
        let mut movement_timer: u32 = 0;
        let mut target_velocity_x: f32 = 0.0;
        let mut facing_right: bool = false; // Fixed: original sprite faces left
        let mut is_idle: bool = true;
        let mut idle_timer: u32 = 0;
        
        // Animation variables
        let mut animation_timer: u32 = 0;
        let mut use_low_frame: bool = false;
        let mut last_animation_frame: bool = false;
        let animation_speed: u32 = 30; // Change frame every 30 ticks (about 0.5 seconds at 60fps)
        let mut is_animating: bool = false;
        let mut animation_check_timer: u32 = 0;
        
        // Flying animation variables
        let mut is_flying: bool = false;
        let mut fly_animation_timer: u32 = 0;
        let mut fly_frame: u32 = 0;
        let fly_animation_speed: u32 = 8; // Change frame every 8 ticks (faster for flying)
        let mut fly_duration: u32 = 0;
        let fly_total_duration: u32 = 90; // Fly animation lasts 1.5 seconds
        
        // Drag and drop variables
        let mut is_dragging: bool = false;
        let mut drag_offset_x: i32 = 0;
        let mut drag_offset_y: i32 = 0;
        
        // Speech bubble variables
        let mut show_bubble: bool = false;
        let mut last_show_bubble: bool = false;
        let mut bubble_timer: u32 = 0;
        let bubble_duration: u32 = 300; // Show bubble for 5 seconds (300 frames at 60fps)
        let mut current_message: String = String::new();
        
        // Screen bounds
        let screen_width = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
        
        // Get bitmap data pointer
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (combined_width * combined_height * 4) as usize);
        
        // Function to render combined image (parrot + optional bubble)
        let render_combined_image = |dest: &mut [u8], parrot_data: &[u8], show_bubble: bool, message: &str| {
            // Clear the entire bitmap
            dest.fill(0);
            
            // Draw bubble first (behind) if showing
            if show_bubble {
                let bubble_offset_y: i32 = utils::BUBBLE_OFFSET_Y;
                let bubble_offset_x: usize = utils::BUBBLE_OFFSET_X as usize;
                for y in 0..scaled_bubble_h {
                    let dest_y_calc = y as i32 + bubble_offset_y;
                    if dest_y_calc < 0 || dest_y_calc >= combined_height as i32 {
                        continue;
                    }
                    let dest_y = dest_y_calc as usize;
                    for x in 0..scaled_bubble_w as usize {
                        let src_idx = (y as usize * scaled_bubble_w as usize + x) * 4;
                        let dest_x = x + bubble_offset_x;
                        if dest_x >= combined_width as usize {
                            continue;
                        }
                        let dest_idx = (dest_y * combined_width as usize + dest_x) * 4;
                        dest[dest_idx + 0] = bubble_bitmap_data[src_idx + 0];
                        dest[dest_idx + 1] = bubble_bitmap_data[src_idx + 1];
                        dest[dest_idx + 2] = bubble_bitmap_data[src_idx + 2];
                        dest[dest_idx + 3] = bubble_bitmap_data[src_idx + 3];
                    }
                }
                // Render text in bubble using fontdue
                if !message.is_empty() {
                    // Split message into words and wrap after every 4 words
                    let words: Vec<&str> = message.split_whitespace().collect();
                    let mut lines: Vec<String> = Vec::new();
                    let mut i = 0;
                    while i < words.len() {
                        let end = (i + 2).min(words.len());
                        let line = words[i..end].join(" ");
                        lines.push(line);
                        i += 2;
                    }
                    let mut pen_y = (bubble_offset_y + utils::BUBBLE_TEXT_START_Y).max(0) as i32;
                    for (line_idx, line) in lines.iter().enumerate() {
                        let mut pen_x = bubble_offset_x as i32 + utils::BUBBLE_TEXT_START_X;
                        let mut char_count = 0;
                        for ch in line.chars() {
                            let font_size = if char_count < 2 { utils::FONT_SIZE_HEAD } else { utils::FONT_SIZE_MAIN };
                            let (metrics, bitmap) = font.rasterize(ch, font_size);
                            for y in 0..metrics.height {
                                for x in 0..metrics.width {
                                    let alpha = bitmap[y * metrics.width + x];
                                    if alpha > 32 {
                                        let dest_x = pen_x + x as i32;
                                        let dest_y = pen_y + y as i32;
                                        if dest_x >= 0 && dest_x < combined_width as i32 && dest_y >= 0 && dest_y < combined_height as i32 {
                                            let idx = (dest_y as usize * combined_width as usize + dest_x as usize) * 4;
                                            dest[idx + 0] = 0;
                                            dest[idx + 1] = 0;
                                            dest[idx + 2] = 0;
                                            dest[idx + 3] = alpha;
                                        }
                                    }
                                }
                            }
                            // Add extra spacing after CJK characters or dash
                            if ch >= '\u{4E00}' && ch <= '\u{9FFF}' {
                                pen_x += metrics.advance_width as i32 + 4; // CJK: add extra space
                            } else if ch == '-' || ch == ' ' {
                                pen_x += metrics.advance_width as i32 + 6; // dash/space: add more space
                            } else {
                                pen_x += metrics.advance_width as i32;
                            }
                            char_count += 1;
                        }
                        if line_idx == 0 {
                            pen_y += utils::FIRST_LINE_SPACING;
                        } else {
                            pen_y += utils::OTHER_LINE_SPACING;
                        }
                    }
                }
            }
            
            // Draw parrot on top (in front of bubble)
            let parrot_y_offset = vertical_padding as usize;
            let parrot_x_offset = scaled_bubble_w as usize;
            for y in 0..scaled_h as usize {
                for x in 0..scaled_w as usize {
                    let src_idx = (y * scaled_w as usize + x) * 4;
                    let dest_idx = ((y + parrot_y_offset) * combined_width as usize + (x + parrot_x_offset)) * 4;
                    dest[dest_idx + 0] = parrot_data[src_idx + 0]; // B
                    dest[dest_idx + 1] = parrot_data[src_idx + 1]; // G
                    dest[dest_idx + 2] = parrot_data[src_idx + 2]; // R
                    dest[dest_idx + 3] = parrot_data[src_idx + 3]; // A
                }
            }
            

        };
        
        // Initialize with normal frame using combined rendering
        render_combined_image(dest, &normal_bitmap_data, false, "");
        
        loop {
            // Scheduler tick: check if a reminder should be queued
            scheduler::tick();

            // If a reminder is ready, make the parrot jump to signal
            if scheduler::has_message_ready() && !is_dragging && !show_bubble && position_y >= (screen_height as f32 - scaled_h as f32 - 1.0) {
                // Simulate a jump by setting upward velocity
                velocity_y = -12.0;
            }

            // Process Windows messages
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE).as_bool() {
                match msg.message {
                    WM_QUIT => {
                        // Cleanup resources before exiting
                        SelectObject(mem_dc, old_bitmap);
                        let _ = DeleteObject(h_bitmap);
                        let _ = DeleteDC(mem_dc);
                        ReleaseDC(HWND(0), screen_dc);
                        return;
                    }
                    WM_LBUTTONDOWN => {
                        // Check if click is within parrot bounds
                        let mut cursor_pos = POINT { x: 0, y: 0 };
                        let _ = GetCursorPos(&mut cursor_pos);
                        
                        let parrot_left = pt_dst.x;
                        let parrot_right = pt_dst.x + combined_width as i32;
                        let parrot_top = pt_dst.y;
                        let parrot_bottom = pt_dst.y + combined_height as i32;
                        
                        if cursor_pos.x >= parrot_left && cursor_pos.x <= parrot_right &&
                           cursor_pos.y >= parrot_top && cursor_pos.y <= parrot_bottom {
                            is_dragging = true;
                            drag_offset_x = cursor_pos.x - pt_dst.x;
                            drag_offset_y = cursor_pos.y - pt_dst.y;
                            // Stop physics when dragging
                            velocity_x = 0.0;
                            velocity_y = 0.0;
                            target_velocity_x = 0.0;
                            is_idle = true;
                        }
                    }
                    WM_LBUTTONUP => {
                        if is_dragging {
                            is_dragging = false;
                            // Reset timers when dropped
                            movement_timer = 0;
                            idle_timer = 0;
                            // Start flying animation
                            is_flying = true;
                            fly_duration = 0;
                            fly_frame = 0;
                            fly_animation_timer = 0;
                        }
                    }
                    WM_RBUTTONDOWN => {
                        // Check if right-click is within parrot bounds
                        let mut cursor_pos = POINT { x: 0, y: 0 };
                        GetCursorPos(&mut cursor_pos).expect("Failed to get cursor position");
                        
                        let parrot_left = pt_dst.x;
                        let parrot_right = pt_dst.x + combined_width as i32;
                        let parrot_top = pt_dst.y;
                        let parrot_bottom = pt_dst.y + combined_height as i32;
                        
                        if cursor_pos.x >= parrot_left && cursor_pos.x <= parrot_right &&
                           cursor_pos.y >= parrot_top && cursor_pos.y <= parrot_bottom {
                            // Show reminder if available, else fallback to random message
                            if scheduler::has_message_ready() {
                                if let Some(reminder) = scheduler::get_message() {
                                    show_bubble = true;
                                    bubble_timer = 0;
                                    current_message = reminder;
                                    velocity_x = 0.0;
                                    velocity_y = 0.0;
                                    target_velocity_x = 0.0;
                                    is_idle = true;
                                    is_flying = false;
                                    fly_duration = 0;
                                    fly_frame = 0;
                                    fly_animation_timer = 0;
                                }
                            } else {
                                // Fallback to random message
                                show_bubble = !show_bubble;
                                bubble_timer = 0;
                                if show_bubble {
                                    let random_index = (rng.next_f32() * messages.len() as f32) as usize;
                                    current_message = messages[random_index].clone();
                                    velocity_x = 0.0;
                                    velocity_y = 0.0;
                                    target_velocity_x = 0.0;
                                    is_idle = true;
                                    is_flying = false;
                                    fly_duration = 0;
                                    fly_frame = 0;
                                    fly_animation_timer = 0;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
            
            // Check mouse state and update position if dragging
            if is_dragging {
                let mut cursor_pos = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut cursor_pos);
                
                let new_x = cursor_pos.x - drag_offset_x;
                let new_y = cursor_pos.y - drag_offset_y;
                
                // Keep within screen bounds
                let screen_right = screen_width - combined_width as i32;
                let screen_bottom = screen_height - scaled_h as i32;
                
                position_x = new_x.max(0).min(screen_right) as f32;
                position_y = new_y.max(0).min(screen_bottom) as f32;
                
                // Check if left mouse button is still pressed
                let left_button_state = windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                    windows::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON.0 as i32
                );
                
                // If left button is not pressed, stop dragging
                if (left_button_state & -0x8000) == 0 {
                    is_dragging = false;
                    movement_timer = 0;
                    idle_timer = 0;
                    // Start flying animation
                    is_flying = true;
                    fly_duration = 0;
                    fly_frame = 0;
                    fly_animation_timer = 0;
                }
            }

            // Handle speech bubble timer
            if show_bubble {
                bubble_timer += 1;
                if bubble_timer >= bubble_duration {
                    show_bubble = false;
                    bubble_timer = 0;
                }
            }
            
            // Only do physics and movement if not being dragged and no bubble is shown
            if !is_dragging && !show_bubble {
                // Fixed random movement system
                movement_timer += 1;
                idle_timer += 1;
                
                if is_idle {
                    // In idle state - randomly decide to start moving
                    let idle_duration = 180 + (rng.next_f32() * 600.0) as u32; // Random idle time 180-780 frames (3-13 seconds)
                    if idle_timer > idle_duration {
                        is_idle = false;
                        idle_timer = 0;
                        movement_timer = 0;
                        // Generate new random horizontal velocity and duration
                        let speed_multiplier = 0.5 + rng.next_f32() * 2.5; // Random speed 0.5-3.0
                        let direction = if rng.next_f32() > 0.5 { 1.0 } else { -1.0 }; // 50/50 chance
                        target_velocity_x = direction * speed_multiplier;
                    }
                } else {
                    // In movement state - randomly decide to stop
                    let movement_duration = 30 + (rng.next_f32() * 90.0) as u32; // Random movement time 30-120 frames (0.5-2 seconds)
                    if movement_timer > movement_duration {
                        is_idle = true;
                        target_velocity_x = 0.0;
                        movement_timer = 0;
                        idle_timer = 0;
                    }
                }
                
                // Smoothly interpolate to target velocity
                velocity_x += (target_velocity_x - velocity_x) * 0.1;
                
                // Physics update
                velocity_y += gravity;
                position_y += velocity_y;
                position_x += velocity_x;

                let screen_bottom = screen_height as f32 - scaled_h as f32;
                let screen_right = screen_width as f32 - combined_width as f32;

                // Floor collision
                if position_y >= screen_bottom {
                    position_y = screen_bottom;
                    velocity_y = -velocity_y * 0.7; // Add some bounce
                }
                
                // Wall collisions
                if position_x <= 0.0 {
                    position_x = 0.0;
                    velocity_x = -velocity_x * 0.8;
                    target_velocity_x = -target_velocity_x * 0.8;
                } else if position_x >= screen_right {
                    position_x = screen_right;
                    velocity_x = -velocity_x * 0.8;
                    target_velocity_x = -target_velocity_x * 0.8;
                }
            }
            
            // Handle flying animation (takes priority over other animations)
            if is_flying {
                fly_duration += 1;
                fly_animation_timer += 1;
                
                // Cycle through fly frames: 0 -> 1 -> 2 -> 1 -> 0 -> 1 -> 2 -> 1...
                if fly_animation_timer >= fly_animation_speed {
                    fly_animation_timer = 0;
                    fly_frame = (fly_frame + 1) % 4; // 0,1,2,1,0,1,2,1...
                }
                
                // End flying animation after duration
                if fly_duration >= fly_total_duration {
                    is_flying = false;
                    fly_duration = 0;
                    fly_frame = 0;
                    fly_animation_timer = 0;
                }
            }
            // Handle idle animation (only if not flying)
            else if is_idle {
                animation_check_timer += 1;
                
                // Check every 60 frames (1 second) if we should start/stop animating
                if animation_check_timer >= 60 {
                    animation_check_timer = 0;
                    // 10% chance to start animating, or stop if already animating
                    if !is_animating {
                        is_animating = rng.next_f32() < 0.1; // 10% chance to start
                    } else {
                        is_animating = rng.next_f32() < 0.7; // 70% chance to continue (average 3-4 seconds)
                    }
                }
                
                if is_animating {
                    animation_timer += 1;
                    if animation_timer >= animation_speed {
                        animation_timer = 0;
                        use_low_frame = !use_low_frame;
                    }
                } else {
                    // Not animating - use normal frame
                    use_low_frame = false;
                    animation_timer = 0;
                }
            } else {
                // When moving, always use normal frame and reset animation state
                use_low_frame = false;
                animation_timer = 0;
                is_animating = false;
                animation_check_timer = 0;
            }
            
            // Update facing direction
            let new_facing_right = velocity_x > 0.1;
            let need_update = new_facing_right != facing_right || use_low_frame != last_animation_frame || is_flying || show_bubble != last_show_bubble;
            
            if need_update {
                facing_right = new_facing_right;
                last_animation_frame = use_low_frame;
                last_show_bubble = show_bubble;
                
                // Determine which parrot image to use
                let parrot_data = if is_flying {
                    // Flying animation takes priority
                    let current_frame = match fly_frame {
                        0 => 0, // parrot1.png
                        1 => 1, // parrot2.png
                        2 => 2, // parrot3.png
                        3 => 1, // parrot2.png (back to middle)
                        _ => 0,
                    };
                    
                    if facing_right {
                        match current_frame {
                            0 => &fly1_bitmap_data,
                            1 => &fly2_bitmap_data,
                            2 => &fly3_bitmap_data,
                            _ => &fly1_bitmap_data,
                        }
                    } else {
                        match current_frame {
                            0 => &flipped_fly1_bits,
                            1 => &flipped_fly2_bits,
                            2 => &flipped_fly3_bits,
                            _ => &flipped_fly1_bits,
                        }
                    }
                } else if facing_right {
                    // Normal idle/movement animation
                    if use_low_frame {
                        &low_bitmap_data
                    } else {
                        &normal_bitmap_data
                    }
                } else {
                    // Going left - use flipped version
                    if use_low_frame {
                        &flipped_low_bits
                    } else {
                        &flipped_normal_bits
                    }
                };
                
                // Render combined image
                render_combined_image(dest, parrot_data, show_bubble, &current_message);
            }

            let new_y = position_y.round() as i32;
            let new_x = position_x.round() as i32;

            if new_y != last_drawn_y || new_x != last_drawn_x || need_update {
                pt_dst.y = new_y - vertical_padding as i32;
                pt_dst.x = new_x;
                last_drawn_y = new_y;
                last_drawn_x = new_x;

                let result = UpdateLayeredWindow(
                    hwnd,
                    screen_dc,
                    Some(&pt_dst),
                    Some(&size),
                    mem_dc,
                    Some(&pt_src),
                    utils::COLOR,
                    Some(&blend),
                    ULW_ALPHA,
                );

                if result.is_err() {
                    eprintln!("UpdateLayeredWindow failed: {:?}", result);
                }
            }

            thread::sleep(Duration::from_millis(16));
        }
    }
}