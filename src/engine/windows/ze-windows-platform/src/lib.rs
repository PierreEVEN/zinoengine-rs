use crate::cursor::WindowsCursor;
use crate::utils::utf8_to_utf16;
use crate::window::WindowsWindow;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::os::raw::c_short;
use std::ptr::null;
use std::sync::{Arc, Weak};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    GetLastError, BOOL, HINSTANCE, HWND, LPARAM, LRESULT, NO_ERROR, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, BLACK_BRUSH, HBRUSH, HDC,
    HMONITOR, MONITORINFO,
};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::*;
use ze_core::maths::{RectI32, Vec2i32};
use ze_platform::{
    Cursor, Error, Message, Monitor, MouseButton, Platform, SystemCursor, Window, WindowFlagBits,
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
        Self(hwnd)
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

            let win_class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: size_of::<usize>() as i32,
                cbWndExtra: 0,
                hInstance: Default::default(),
                hIcon: Default::default(),
                hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszMenuName: Default::default(),
                lpszClassName: PCWSTR(utf8_to_utf16(WIN_CLASS_NAME).as_ptr()),
                hIconSm: Default::default(),
            };
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
                null(),
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
                        self.mouse_position(),
                    ));
                }
                WM_MBUTTONDOWN => {
                    message_queue.push_back(Message::MouseButtonDown(
                        window.clone(),
                        MouseButton::Middle,
                        self.mouse_position(),
                    ));
                }
                WM_RBUTTONDOWN => {
                    message_queue.push_back(Message::MouseButtonDown(
                        window.clone(),
                        MouseButton::Right,
                        self.mouse_position(),
                    ));
                }

                WM_LBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Left,
                        self.mouse_position(),
                    ));
                }
                WM_MBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Middle,
                        self.mouse_position(),
                    ));
                }
                WM_RBUTTONUP => {
                    message_queue.push_back(Message::MouseButtonUp(
                        window.clone(),
                        MouseButton::Right,
                        self.mouse_position(),
                    ));
                }

                WM_LBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Left,
                        self.mouse_position(),
                    ));
                }
                WM_MBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Middle,
                        self.mouse_position(),
                    ));
                }
                WM_RBUTTONDBLCLK => {
                    message_queue.push_back(Message::MouseButtonDoubleClick(
                        window.clone(),
                        MouseButton::Right,
                        self.mouse_position(),
                    ));
                }
                WM_MOUSEWHEEL => {
                    message_queue.push_back(Message::MouseWheel(
                        window.clone(),
                        (ze_win_hiword!(wparam.0) as c_short as f32) / (WHEEL_DELTA as f32),
                        self.mouse_position(),
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
    GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).unwrap();

    let monitors = (userdata.0 as *mut Vec<Monitor>)
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
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            None
        }
    }

    fn create_window(
        &self,
        name: &str,
        mut width: u32,
        mut height: u32,
        mut x: i32,
        mut y: i32,
        flags: WindowFlags,
    ) -> Result<Arc<dyn Window>, Error> {
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
                return Err(Error::Unknown);
            }

            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

            ShowWindow(
                hwnd,
                if flags.contains(WindowFlagBits::Maximized) {
                    SW_SHOWMAXIMIZED
                } else {
                    SW_SHOW
                },
            );

            if flags.contains(WindowFlagBits::Maximized) {
                let mut client_rect = RECT::default();
                GetClientRect(hwnd, &mut client_rect);

                let mut position = POINT {
                    x: client_rect.left,
                    y: client_rect.top,
                };
                ClientToScreen(hwnd, &mut position);

                x = position.x;
                y = position.y;
                width = (client_rect.right - client_rect.left) as u32;
                height = (client_rect.bottom - client_rect.top) as u32;
            }

            let window = WindowsWindow::new(hwnd, width, height, x, y, style, ex_style);
            self.window_map
                .lock()
                .insert(hwnd.into(), Arc::downgrade(&window));

            Ok(window)
        }
    }

    fn create_system_cursor(&self, cursor: SystemCursor) -> Box<dyn Cursor> {
        let name = match cursor {
            SystemCursor::No => IDC_NO,
            SystemCursor::Crosshair => IDC_CROSS,
            SystemCursor::Ibeam => IDC_IBEAM,
            SystemCursor::Arrow => IDC_ARROW,
            SystemCursor::Hand => IDC_HAND,
            SystemCursor::SizeAll => IDC_SIZEALL,
            SystemCursor::SizeNorthEastSouthWest => IDC_SIZENESW,
            SystemCursor::SizeNorthSouth => IDC_SIZENS,
            SystemCursor::SizeNorthWestSouthEast => IDC_SIZENWSE,
            SystemCursor::SizeWestEast => IDC_SIZEWE,
            SystemCursor::Wait | SystemCursor::WaitArrow => IDC_WAIT,
        };

        let cursor = unsafe { LoadCursorW(HINSTANCE::default(), name) };
        Box::new(WindowsCursor::new(cursor.unwrap()))
    }

    fn set_cursor(&self, cursor: Option<&dyn Cursor>) {
        let mut win_cursor = HCURSOR::default();

        if let Some(cursor) = cursor {
            let cursor = cursor.downcast_ref::<WindowsCursor>().unwrap();
            win_cursor = cursor.cursor;
        }

        unsafe {
            SetCursor(win_cursor);
        }
    }

    fn mouse_position(&self) -> Vec2i32 {
        let mut pos = POINT::default();
        unsafe { GetCursorPos(&mut pos) };
        Vec2i32::new(pos.x, pos.y)
    }

    fn monitor_count(&self) -> usize {
        self.monitors.lock().len()
    }

    fn monitor(&self, index: usize) -> Monitor {
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

mod cursor;
mod utils;
mod window;