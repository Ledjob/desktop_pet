use image::GenericImageView;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{RGBQUAD, RGBTRIPLE};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CYSCREEN};
use std::{ffi::c_void, ptr::null_mut, thread, time::Duration};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
            CreateDIBSection, DIB_RGB_COLORS, GetDC, SelectObject, HBITMAP, HDC, DeleteDC, DeleteObject, ReleaseDC,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, RegisterClassW, ShowWindow, UpdateLayeredWindow, ULW_ALPHA,
            CS_HREDRAW, CS_VREDRAW, SW_SHOW, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
            PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE, WM_QUIT,
        },
    },
};

const COLOR: COLORREF = windows::Win32::Foundation::COLORREF(0); // transparent color for the background

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
            windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// Simple linear congruential generator for better randomness
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new() -> Self {
        Self { 
            state: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        }
    }
    
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }
    
    fn next_f32(&mut self) -> f32 {
        (self.next() & 0xFFFFFF) as f32 / 0xFFFFFF as f32
    }
}

fn main() {
    unsafe {
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        // Load image
        let image = image::open("parrot.png").expect("parrot.png not found").to_rgba8();
        let (img_w, img_h) = image.dimensions();
        let image_data = image.as_flat_samples().samples;

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
        let scaled_w = img_w / 4;
        let scaled_h = img_h / 4;

        // Create transparent layered window - fullscreen
        let screen_width = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
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

        ShowWindow(hwnd, SW_SHOW);

        // Create memory DC and DIB section
        let screen_dc: HDC = GetDC(HWND(0));
        let mem_dc: HDC = CreateCompatibleDC(screen_dc);

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: scaled_w as i32,
                biHeight: -(scaled_h as i32), // top-down
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

        // Copy RGBA to BGRA with scaling (simple nearest neighbor)
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (scaled_w * scaled_h * 4) as usize);
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_x = (x * 4) as usize;
                let src_y = (y * 4) as usize;
                let src_idx = (src_y * img_w as usize + src_x) * 4;
                let dest_idx = (y * scaled_w + x) as usize * 4;
                
                dest[dest_idx + 0] = image_data[src_idx + 2]; // B
                dest[dest_idx + 1] = image_data[src_idx + 1]; // G
                dest[dest_idx + 2] = image_data[src_idx + 0]; // R
                dest[dest_idx + 3] = image_data[src_idx + 3]; // A
            }
        }

        let mut pt_dst = POINT { x: 300, y: 300 };
        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: scaled_w as i32,
            cy: scaled_h as i32,
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
        let mut rng = SimpleRng::new();
        let mut movement_timer: u32 = 0;
        let mut target_velocity_x: f32 = 0.0;
        let mut facing_right: bool = false; // Fixed: original sprite faces left
        let mut is_idle: bool = true;
        let mut idle_timer: u32 = 0;
        
        // Screen bounds
        let screen_width = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
        
        // Create flipped bitmap for right-facing parrot
        let mut flipped_bits: Vec<u8> = vec![0; (scaled_w * scaled_h * 4) as usize];
        for y in 0..scaled_h {
            for x in 0..scaled_w {
                let src_idx = (y * scaled_w + x) as usize * 4;
                let flipped_x = scaled_w - 1 - x;
                let dest_idx = (y * scaled_w + flipped_x) as usize * 4;
                
                flipped_bits[dest_idx + 0] = dest[src_idx + 0]; // B
                flipped_bits[dest_idx + 1] = dest[src_idx + 1]; // G
                flipped_bits[dest_idx + 2] = dest[src_idx + 2]; // R
                flipped_bits[dest_idx + 3] = dest[src_idx + 3]; // A
            }
        }

        loop {
            // Process Windows messages
            while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    // Cleanup resources before exiting
                    SelectObject(mem_dc, old_bitmap);
                    DeleteObject(h_bitmap);
                    DeleteDC(mem_dc);
                    ReleaseDC(HWND(0), screen_dc);
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

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
            let screen_right = screen_width as f32 - scaled_w as f32;

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
            
            // Fixed facing direction logic
            let new_facing_right = velocity_x > 0.1;
            if new_facing_right != facing_right {
                facing_right = new_facing_right;
                // Update bitmap data based on facing direction
                if facing_right {
                    // Going right - use original (since we need to flip the logic)
                    for y in 0..scaled_h {
                        for x in 0..scaled_w {
                            let src_x = (x * 4) as usize;
                            let src_y = (y * 4) as usize;
                            let src_idx = (src_y * img_w as usize + src_x) * 4;
                            let dest_idx = (y * scaled_w + x) as usize * 4;
                            
                            dest[dest_idx + 0] = image_data[src_idx + 2]; // B
                            dest[dest_idx + 1] = image_data[src_idx + 1]; // G
                            dest[dest_idx + 2] = image_data[src_idx + 0]; // R
                            dest[dest_idx + 3] = image_data[src_idx + 3]; // A
                        }
                    }
                } else {
                    // Going left - use flipped version
                    dest.copy_from_slice(&flipped_bits);
                }
            }

            let new_y = position_y.round() as i32;
            let new_x = position_x.round() as i32;

            if new_y != last_drawn_y || new_x != last_drawn_x {
                pt_dst.y = new_y;
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
                    COLOR,
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