use std::os::raw::c_char;
use std::ptr;

const DEFAULT_BUFFER_CAPACITY_IN_CHARS: usize = 1024;

/// Simple buffer to convert from Rust strings to C-null terminated UTF-8 strings
pub struct StrBuffer {
    buffer: Vec<u8>,
}

impl StrBuffer {
    pub fn convert(&mut self, text: &str) -> *const c_char {
        unsafe {
            ptr::copy_nonoverlapping(text.as_ptr(), self.buffer.as_mut_ptr(), text.len());
        }
        self.buffer[text.len()] = b'\0';
        self.buffer.as_ptr() as *const c_char
    }
}

impl Default for StrBuffer {
    fn default() -> Self {
        let buffer = [0; DEFAULT_BUFFER_CAPACITY_IN_CHARS];

        Self {
            buffer: Vec::from(buffer),
        }
    }
}
