use image::GenericImageView;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;
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
            CS_HREDRAW, CS_VREDRAW, SW_SHOW, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP,
            PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE, WM_QUIT,
            WM_LBUTTONDOWN, WM_LBUTTONUP, GetCursorPos,
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
        
        // Load all images
        let image_normal = image::open("assets/parrot.png").expect("parrot.png not found").to_rgba8();
        let image_low = image::open("assets/parrot_low.png").expect("parrot_low.png not found").to_rgba8();
        let image_fly1 = image::open("assets/parrot1.png").expect("parrot1.png not found").to_rgba8();
        let image_fly2 = image::open("assets/parrot2.png").expect("parrot2.png not found").to_rgba8();
        let image_fly3 = image::open("assets/parrot3.png").expect("parrot3.png not found").to_rgba8();
        
        let (img_w, img_h) = image_normal.dimensions();
        let image_normal_data = image_normal.as_flat_samples().samples;
        let image_low_data = image_low.as_flat_samples().samples;
        let image_fly1_data = image_fly1.as_flat_samples().samples;
        let image_fly2_data = image_fly2.as_flat_samples().samples;
        let image_fly3_data = image_fly3.as_flat_samples().samples;

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
            WS_EX_LAYERED | WS_EX_TOOLWINDOW,
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
        
        // Screen bounds
        let screen_width = GetSystemMetrics(windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN);
        
        // Get bitmap data pointer
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (scaled_w * scaled_h * 4) as usize);
        
        // Initialize with normal frame
        dest.copy_from_slice(&normal_bitmap_data);

        loop {
            // Process Windows messages
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE).as_bool() {
                match msg.message {
                    WM_QUIT => {
                        // Cleanup resources before exiting
                        SelectObject(mem_dc, old_bitmap);
                        DeleteObject(h_bitmap);
                        DeleteDC(mem_dc);
                        ReleaseDC(HWND(0), screen_dc);
                        return;
                    }
                    WM_LBUTTONDOWN => {
                        // Check if click is within parrot bounds
                        let mut cursor_pos = POINT { x: 0, y: 0 };
                        GetCursorPos(&mut cursor_pos);
                        
                        let parrot_left = pt_dst.x;
                        let parrot_right = pt_dst.x + scaled_w as i32;
                        let parrot_top = pt_dst.y;
                        let parrot_bottom = pt_dst.y + scaled_h as i32;
                        
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
                    _ => {}
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            
            // Check mouse state and update position if dragging
            if is_dragging {
                let mut cursor_pos = POINT { x: 0, y: 0 };
                GetCursorPos(&mut cursor_pos);
                
                let new_x = cursor_pos.x - drag_offset_x;
                let new_y = cursor_pos.y - drag_offset_y;
                
                // Keep within screen bounds
                let screen_right = screen_width - scaled_w as i32;
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

            // Only do physics and movement if not being dragged
            if !is_dragging {
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
            let need_update = new_facing_right != facing_right || use_low_frame != last_animation_frame || is_flying;
            
            if need_update {
                facing_right = new_facing_right;
                last_animation_frame = use_low_frame;
                
                // Update bitmap data based on state and facing direction
                if is_flying {
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
                            0 => dest.copy_from_slice(&fly1_bitmap_data),
                            1 => dest.copy_from_slice(&fly2_bitmap_data),
                            2 => dest.copy_from_slice(&fly3_bitmap_data),
                            _ => dest.copy_from_slice(&fly1_bitmap_data),
                        }
                    } else {
                        match current_frame {
                            0 => dest.copy_from_slice(&flipped_fly1_bits),
                            1 => dest.copy_from_slice(&flipped_fly2_bits),
                            2 => dest.copy_from_slice(&flipped_fly3_bits),
                            _ => dest.copy_from_slice(&flipped_fly1_bits),
                        }
                    }
                } else if facing_right {
                    // Normal idle/movement animation
                    if use_low_frame {
                        dest.copy_from_slice(&low_bitmap_data);
                    } else {
                        dest.copy_from_slice(&normal_bitmap_data);
                    }
                } else {
                    // Going left - use flipped version
                    if use_low_frame {
                        dest.copy_from_slice(&flipped_low_bits);
                    } else {
                        dest.copy_from_slice(&flipped_normal_bits);
                    }
                }
            }

            let new_y = position_y.round() as i32;
            let new_x = position_x.round() as i32;

            if new_y != last_drawn_y || new_x != last_drawn_x || need_update {
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