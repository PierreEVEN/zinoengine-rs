use crate::utf8_to_utf16;
use raw_window_handle::{RawWindowHandle, Win32Handle};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use ze_core::maths::Vec2i32;
use ze_platform::Window;

pub struct WindowsWindow {
    hwnd: HWND,
    width: AtomicU32,
    height: AtomicU32,
    x: AtomicI32,
    y: AtomicI32,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
}

impl WindowsWindow {
    pub fn new(
        hwnd: HWND,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        style: WINDOW_STYLE,
        ex_style: WINDOW_EX_STYLE,
    ) -> Arc<WindowsWindow> {
        Arc::new(WindowsWindow {
            hwnd,
            width: AtomicU32::new(width),
            height: AtomicU32::new(height),
            x: AtomicI32::new(x),
            y: AtomicI32::new(y),
            style,
            ex_style,
        })
    }

    pub fn send_window_message(&self, msg: u32, _: WPARAM, lparam: LPARAM) {
        match msg {
            WM_SIZE => {
                let width = ze_win_loword!(lparam.0);
                let height = ze_win_hiword!(lparam.0);
                self.width.store(width as u32, Ordering::SeqCst);
                self.height.store(height as u32, Ordering::SeqCst);
            }
            WM_MOVE => {
                let x = ze_win_loword!(lparam.0);
                let y = ze_win_hiword!(lparam.0);
                self.x.store(x as i32, Ordering::SeqCst);
                self.y.store(y as i32, Ordering::SeqCst);
            }
            _ => {}
        }
    }
}

impl Drop for WindowsWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

impl Window for WindowsWindow {
    fn set_position(&self, position: Vec2i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND::default(),
                position.x,
                position.y,
                self.width.load(Ordering::SeqCst) as i32,
                self.height.load(Ordering::SeqCst) as i32,
                SET_WINDOW_POS_FLAGS(0),
            )
        };
        self.x.store(position.x, Ordering::SeqCst);
        self.y.store(position.y, Ordering::SeqCst);
    }

    fn set_size(&self, width: u32, height: u32) {
        unsafe {
            let mut initial_rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            AdjustWindowRectEx(&mut initial_rect, self.style, false, self.ex_style);

            self.width.store(width, Ordering::SeqCst);
            self.height.store(height, Ordering::SeqCst);

            SetWindowPos(
                self.hwnd,
                HWND::default(),
                self.x.load(Ordering::SeqCst) as i32,
                self.y.load(Ordering::SeqCst) as i32,
                self.width.load(Ordering::SeqCst) as i32,
                self.height.load(Ordering::SeqCst) as i32,
                SET_WINDOW_POS_FLAGS(0),
            )
        };
    }

    fn set_title(&self, title: &str) {
        let title = utf8_to_utf16(title);
        unsafe {
            SetWindowTextW(self.hwnd, PCWSTR(title.as_ptr()));
        }
    }

    fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
    }

    fn handle(&self) -> RawWindowHandle {
        let mut handle = Win32Handle::empty();
        handle.hwnd = self.hwnd.0 as *mut std::ffi::c_void;
        RawWindowHandle::Win32(handle)
    }

    fn width(&self) -> u32 {
        self.width.load(Ordering::SeqCst)
    }

    fn height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    fn position(&self) -> Vec2i32 {
        Vec2i32::new(self.x.load(Ordering::SeqCst), self.y.load(Ordering::SeqCst))
    }
}
