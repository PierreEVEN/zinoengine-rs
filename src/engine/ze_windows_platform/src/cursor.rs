use windows::Win32::UI::WindowsAndMessaging::{DestroyCursor, HCURSOR};
use ze_platform::Cursor;

pub struct WindowsCursor {
    pub cursor: HCURSOR,
}

impl WindowsCursor {
    pub fn new(cursor: HCURSOR) -> Self {
        Self { cursor }
    }
}

impl Drop for WindowsCursor {
    fn drop(&mut self) {
        unsafe {
            DestroyCursor(self.cursor);
        }
    }
}

impl Cursor for WindowsCursor {}
