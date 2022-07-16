use enumflags2::{bitflags, BitFlags};
use raw_window_handle::RawWindowHandle;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Weak};
use ze_core::downcast_rs::{impl_downcast, Downcast};
use ze_core::maths::{RectI32, Vec2i32};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum WindowFlagBits {
    Maximized = 1 << 1,
    Borderless = 1 << 2,
    Resizable = 1 << 3,
}
pub type WindowFlags = BitFlags<WindowFlagBits>;

pub enum SystemCursor {
    No,
    Crosshair,
    Ibeam,
    Arrow,
    Hand,
    SizeAll,
    SizeNorthEastSouthWest,
    SizeNorthSouth,
    SizeNorthWestSouthEast,
    SizeWestEast,
    Wait,
    WaitArrow,
}

#[derive(Copy, Clone)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

pub trait Cursor: Downcast {}
impl_downcast!(Cursor);

pub trait Window: Downcast + Send + Sync {
    fn set_position(&self, pos: Vec2i32);
    fn set_size(&self, width: u32, height: u32);
    fn set_title(&self, title: &str);
    fn show(&self);

    fn get_handle(&self) -> RawWindowHandle;
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
    fn get_position(&self) -> Vec2i32;
}
impl_downcast!(Window);

pub enum Message {
    WindowClosed(Weak<dyn Window>),
    WindowResized(Weak<dyn Window>, u32, u32),

    MouseButtonDown(Weak<dyn Window>, MouseButton, Vec2i32),
    MouseButtonUp(Weak<dyn Window>, MouseButton, Vec2i32),
    MouseButtonDoubleClick(Weak<dyn Window>, MouseButton, Vec2i32),
    MouseWheel(Weak<dyn Window>, f32, Vec2i32),
}

#[derive(Copy, Clone)]
pub struct Monitor {
    pub bounds: RectI32,
    pub work_bounds: RectI32,
    pub dpi: f32,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Error {
    Unknown,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

/// Trait describing a platform, supporting window creation, event handling etc
pub trait Platform: Send + Sync {
    fn poll_event(&self) -> Option<Message>;
    fn create_window(
        &self,
        name: &str,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        flags: WindowFlags,
    ) -> Result<Arc<dyn Window>, Error>;

    fn create_system_cursor(&self, cursor: SystemCursor) -> Box<dyn Cursor>;
    fn get_mouse_position(&self) -> Vec2i32;

    fn get_monitor_count(&self) -> usize;
    fn get_monitor(&self, index: usize) -> Monitor;
}
