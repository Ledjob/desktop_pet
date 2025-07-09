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

        // Create transparent layered window
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(to_wide("Parrot Pet").as_ptr()),
            WS_POPUP,
            300,
            300,
            img_w as i32,
            img_h as i32,
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
                biWidth: img_w as i32,
                biHeight: -(img_h as i32), // top-down
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

        // Copy RGBA to BGRA
        let dest = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (img_w * img_h * 4) as usize);
        for i in 0..(img_w * img_h) as usize {
            dest[i * 4 + 0] = image_data[i * 4 + 2]; // B
            dest[i * 4 + 1] = image_data[i * 4 + 1]; // G
            dest[i * 4 + 2] = image_data[i * 4 + 0]; // R
            dest[i * 4 + 3] = image_data[i * 4 + 3]; // A
        }

        let mut pt_dst = POINT { x: 300, y: 300 };
        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: img_w as i32,
            cy: img_h as i32,
        };

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_ALPHA as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        // Basic loop with gravity
        let mut velocity_y: f32 = 0.0;
        let gravity: f32 = 0.5;
        let mut position_y: f32 = pt_dst.y as f32;
        let mut last_drawn_y: i32 = pt_dst.y;
        let mut msg = MSG::default();

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

            // Physics update
            velocity_y += gravity;
            position_y += velocity_y;

            let screen_bottom = screen_height as f32 - img_h as f32;

            // Floor collision
            if position_y >= screen_bottom {
                position_y = screen_bottom;
                velocity_y = -velocity_y * 0.7; // Add some bounce
            }

            let new_y = position_y.round() as i32;

            if new_y != last_drawn_y {
                pt_dst.y = new_y;
                last_drawn_y = new_y;

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