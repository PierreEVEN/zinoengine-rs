use enumflags2::*;
use raw_window_handle::RawWindowHandle;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Weak};
use ze_core::downcast_rs::{impl_downcast, Downcast};
use ze_core::maths::{Point2, RectI32};

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
    fn set_position(&self, pos: Point2<i32>);
    fn set_size(&self, width: u32, height: u32);
    fn set_title(&self, title: &str);
    fn show(&self);

    fn handle(&self) -> RawWindowHandle;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn position(&self) -> Point2<i32>;
}
impl_downcast!(Window);

pub enum Message {
    WindowClosed(Weak<dyn Window>),
    WindowResized(Weak<dyn Window>, u32, u32),

    MouseButtonDown(Weak<dyn Window>, MouseButton, Point2<i32>),
    MouseButtonUp(Weak<dyn Window>, MouseButton, Point2<i32>),
    MouseButtonDoubleClick(Weak<dyn Window>, MouseButton, Point2<i32>),
    MouseWheel(Weak<dyn Window>, f32, Point2<i32>),

    KeyDown(Weak<dyn Window>, KeyCode, u32, bool),
    KeyUp(Weak<dyn Window>, KeyCode, u32, bool),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum KeyCode {
    None,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    LeftShift,
    RightShift,
    Space,
    Backspace,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
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
    fn set_cursor(&self, cursor: Option<&dyn Cursor>);
    fn mouse_position(&self) -> Point2<i32>;

    fn monitor_count(&self) -> usize;
    fn monitor(&self, index: usize) -> Monitor;
}
