use crate::utils::utf8_to_utf16;
use parking_lot::Mutex;
use raw_window_handle::{RawWindowHandle, Win32Handle};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::mem::{size_of, transmute};
use std::ops::Deref;
use std::os::raw::c_short;
use std::ptr::null;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    GetLastError, BOOL, HINSTANCE, HWND, LPARAM, LRESULT, NO_ERROR, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, BLACK_BRUSH, HBRUSH, HDC, HMONITOR,
    MONITORINFO,
};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::*;
use ze_core::maths::{RectI32, Vec2i32, Vec2u32};
use ze_platform::{
    Cursor, Message, Monitor, MouseButton, Platform, SystemCursor, Window, WindowFlagBits,
    WindowFlags,
};

macro_rules! ze_win_loword {
    ($arg:expr) => {
        $arg & 0xffff
    };
}

macro_rules! ze_win_hiword {
    ($arg:expr) => {
        ($arg >> 16) & 0xffff
    };
}

const WIN_CLASS_NAME: &str = "ze_window";

struct HashableHWND(HWND);

impl PartialEq for HashableHWND {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for HashableHWND {}

impl Hash for HashableHWND {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_isize(self.0 .0);
    }
}

impl From<HWND> for HashableHWND {
    fn from(hwnd: HWND) -> Self {
        Self { 0: hwnd }
    }
}

pub struct WindowsPlatform {
    window_map: Mutex<HashMap<HashableHWND, Weak<WindowsWindow>>>,
    message_queue: Mutex<VecDeque<Message>>,
    monitors: Mutex<Vec<Monitor>>,
}

impl WindowsPlatform {
    pub fn new() -> Arc<WindowsPlatform> {
        unsafe {
            timeBeginPeriod(1);

            let mut win_class = WNDCLASSEXW::default();
            win_class.cbSize = size_of::<WNDCLASSEXW>() as u32;
            win_class.style = CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS;
            win_class.lpszClassName = PCWSTR(utf8_to_utf16(WIN_CLASS_NAME).as_ptr());
            win_class.hbrBackground = HBRUSH(GetStockObject(BLACK_BRUSH).0);
            win_class.hCursor = LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap();
            win_class.cbClsExtra = size_of::<usize>() as i32;
            win_class.lpfnWndProc = Some(wnd_proc);
            assert_ne!(RegisterClassExW(&win_class), 0);

            let platform = Arc::new(WindowsPlatform {
                window_map: Default::default(),
                message_queue: Mutex::new(VecDeque::new()),
                monitors: Default::default(),
            });

            // Create dummy window to set platform pointer into the WNDCLASS
            {
                let dummy_window = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    PCWSTR(utf8_to_utf16(WIN_CLASS_NAME).as_ptr()),
                    PCWSTR::default(),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    1,
                    1,
                    HWND::default(),
                    HMENU::default(),
                    HINSTANCE::default(),
                    null(),
                );

                SetClassLongPtrW(
                    dummy_window,
                    GET_CLASS_LONG_INDEX(0),
                    (platform.as_ref() as *const WindowsPlatform) as isize,
                );

                DestroyWindow(dummy_window);
            }

            platform.update_monitors();

            platform
        }
    }

    fn update_monitors(&self) {
        let mut monitors = self.monitors.lock();
        monitors.clear();

        unsafe {
            EnumDisplayMonitors(
                HDC::default(),
                std::ptr::null(),
                Some(enum_display_monitors_callback),
                LPARAM((&*monitors as *const _) as isize),
            );
        }
    }

    fn send_window_message(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) {
        let window_map = self.window_map.lock();
        if let Some(window) = window_map.get(&hwnd.into()) {
            let mut message_queue = self.message_queue.lock();
            if let Some(window) = window.upgrade() {
                window.send_window_message(msg, wparam, lparam);
            }
            match msg {
                WM_CLOSE => {
                    message_queue.push_back(Message::WindowClosed(window.clone()));
                }
                WM_SIZE => {
                    message_queue.push_back(Message::WindowResized(
                        window.clone(),
                        ze_win_loword!(lparam.0) as u32,
                        ze_win_hiword!(lparam.0) as u32,
                    ));
                }
                WM_LBUTTONDOWN => {
                    message_queue.push_back(Message::MouseButtonDown(
                        window.clone(),
                        MouseButton::Left,
                        self.get_mouse_position(),
                    ));
                }
                WM_MBUTTONDOWN => {
                    message_queue.push_back(Message::MouseButtonDown(
                        window.clone(),
                        MouseButton::Middle,
                        self.get_mouse_position(),
                    ));
                }
                WM_RBUTTONDOWN => {
                    message_queue.push_back(Message::MouseButtonDown(
                        window.clone(),
                        MouseButton::Right,
                        self.get_mouse_position(),
                    ));
                }

                WM_LBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Left,
                        self.get_mouse_position(),
                    ));
                }
                WM_MBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Middle,
                        self.get_mouse_position(),
                    ));
                }
                WM_RBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Right,
                        self.get_mouse_position(),
                    ));
                }

                WM_LBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Left,
                        self.get_mouse_position(),
                    ));
                }
                WM_MBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Middle,
                        self.get_mouse_position(),
                    ));
                }
                WM_RBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Right,
                        self.get_mouse_position(),
                    ));
                }
                WM_MOUSEWHEEL => {
                    message_queue.push_back(Message::MouseWheel(
                        window.clone(),
                        (ze_win_hiword!(wparam.0) as c_short as f32) / (WHEEL_DELTA as f32),
                        self.get_mouse_position(),
                    ));
                }
                _ => (),
            }
        }
    }
}

unsafe extern "system" fn enum_display_monitors_callback(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    userdata: LPARAM,
) -> BOOL {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: Default::default(),
        rcWork: Default::default(),
        dwFlags: 0,
    };

    GetMonitorInfoW(monitor, &mut info);

    let mut dpi_x = 0;
    let mut dpi_y = 0;
    GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);

    let mut monitors = (userdata.0 as *mut Vec<Monitor>)
        .as_mut()
        .unwrap_unchecked();

    monitors.push(Monitor {
        bounds: RectI32::new(
            info.rcMonitor.left,
            info.rcMonitor.top,
            info.rcMonitor.right - info.rcMonitor.left,
            info.rcMonitor.bottom - info.rcMonitor.top,
        ),
        work_bounds: RectI32::new(
            info.rcWork.left,
            info.rcWork.top,
            info.rcWork.right - info.rcWork.left,
            info.rcWork.bottom - info.rcWork.top,
        ),
        dpi: dpi_x as f32,
    });

    BOOL::from(true)
}

impl Drop for WindowsPlatform {
    fn drop(&mut self) {
        unsafe {
            UnregisterClassW(
                PCWSTR(utf8_to_utf16(WIN_CLASS_NAME).as_ptr()),
                HINSTANCE::default(),
            );
            timeEndPeriod(1);
        }
    }
}

impl Platform for WindowsPlatform {
    fn poll_event(&self) -> Option<Message> {
        let mut message_queue = self.message_queue.lock();
        if let Some(message) = message_queue.pop_front() {
            Some(message)
        } else {
            drop(message_queue);

            unsafe {
                let mut msg = std::mem::zeroed();
                if PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE) != false {
                    TranslateMessage(&mut msg);
                    DispatchMessageW(&mut msg);
                }
            }

            None
        }
    }

    fn create_window(
        &self,
        name: &str,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        flags: WindowFlags,
    ) -> Result<Arc<dyn Window>, ()> {
        let ex_style = WS_EX_LAYERED;
        let mut style = WINDOW_STYLE::default();

        if flags.contains(WindowFlagBits::Borderless) {
            style |= WS_VISIBLE | WS_POPUP;
        } else {
            style |= WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
        }

        if flags.contains(WindowFlagBits::Resizable) {
            style |= WS_THICKFRAME;
        }

        // Rect must be ajusted since Win32 api include window decoration in the width/height
        let mut initial_rect = RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };

        unsafe {
            AdjustWindowRectEx(&mut initial_rect, style, false, ex_style);
            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR(utf8_to_utf16(WIN_CLASS_NAME).as_ptr()),
                PCWSTR(utf8_to_utf16(name).as_ptr()),
                style,
                x + initial_rect.left,
                y + initial_rect.top,
                initial_rect.right - initial_rect.left,
                initial_rect.bottom - initial_rect.top,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                null(),
            );

            if GetLastError() != NO_ERROR {
                return Err(());
            }

            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

            ShowWindow(
                hwnd,
                if flags.contains(WindowFlagBits::Maximized) {
                    SW_MAXIMIZE
                } else {
                    SW_SHOW
                },
            );

            let window = WindowsWindow::new(hwnd, width, height, x, y, style, ex_style);
            self.window_map
                .lock()
                .insert(hwnd.into(), Arc::downgrade(&window));
            return Ok(window);
        };
    }

    fn create_system_cursor(&self, _: SystemCursor) -> Box<dyn Cursor> {
        todo!()
    }

    fn get_mouse_position(&self) -> Vec2i32 {
        let mut pos = POINT::default();
        unsafe { GetCursorPos(&mut pos) };
        Vec2i32::new(pos.x, pos.y)
    }

    fn get_monitor_count(&self) -> usize {
        self.monitors.lock().len()
    }

    fn get_monitor(&self, index: usize) -> Monitor {
        self.monitors.lock()[index]
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let platform = {
        let ptr = GetClassLongPtrW(hwnd, GET_CLASS_LONG_INDEX(0));
        if ptr == 0 {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }

        (ptr as *const WindowsPlatform).as_ref().unwrap_unchecked()
    };

    platform.send_window_message(hwnd, msg, wparam, lparam);
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

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

    fn send_window_message(&self, msg: u32, _: WPARAM, lparam: LPARAM) {
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

    fn get_handle(&self) -> RawWindowHandle {
        let mut handle = Win32Handle::empty();
        handle.hwnd = self.hwnd.0 as *mut std::ffi::c_void;
        RawWindowHandle::Win32(handle)
    }

    fn get_width(&self) -> u32 {
        self.width.load(Ordering::SeqCst)
    }

    fn get_height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    fn get_position(&self) -> Vec2i32 {
        Vec2i32::new(self.x.load(Ordering::SeqCst), self.y.load(Ordering::SeqCst))
    }
}

mod utils;
