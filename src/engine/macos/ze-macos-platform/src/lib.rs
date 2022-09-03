use cocoa::appkit::NSScreen;
use cocoa::base::*;
use cocoa::foundation::NSArray;
use core_graphics::display::{
    CGDirectDisplayID, CGDisplayBounds, CGDisplayCopyDisplayMode, CGDisplayModeGetPixelHeight,
    CGDisplayModeGetPixelWidth, CGDisplayModeRelease, CGDisplayScreenSize, CGGetActiveDisplayList,
};
use parking_lot::Mutex;
use std::sync::Arc;
use ze_platform::{Monitor, Platform};

pub struct MacOSPlatform {
    monitors: Mutex<Vec<Monitor>>,
}

impl MacOSPlatform {
    pub fn new() -> Arc<Self> {
        let platform = Self {
            monitors: Default::default(),
        };
        platform.update_monitors();
        Arc::new(platform)
    }

    fn update_monitors(&self) {
        unsafe {
            let mut monitors = self.monitors.lock();
            monitors.clear();

            let mut active_displays = [CGDirectDisplayID::default(); 256];
            let mut active_display_count = 0;
            CGGetActiveDisplayList(
                active_displays.len() as u32,
                active_displays.as_mut().as_mut_ptr(),
                &mut active_display_count,
            );

            for i in 0..active_display_count {
                let display = active_displays[i as usize];
                let bounds = CGDisplayBounds(display);
                let display_mode = CGDisplayCopyDisplayMode(display);
                let width = CGDisplayModeGetPixelWidth(display_mode);
                let height = CGDisplayModeGetPixelHeight(display_mode);
                let test = CGDisplayScreenSize(display);
                CGDisplayModeRelease(display_mode);
                /*monitors.push(Monitor {
                    bounds: (),
                    work_bounds: (),
                    dpi: (),
                });*/
            }
        }
    }
}

impl Platform for MacOSPlatform {
    fn poll_event(&self) -> Option<ze_platform::Message> {
        todo!()
    }

    fn create_window(
        &self,
        name: &str,
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        flags: ze_platform::WindowFlags,
    ) -> Result<Arc<dyn ze_platform::Window>, ze_platform::Error> {
        todo!()
    }

    fn create_system_cursor(
        &self,
        cursor: ze_platform::SystemCursor,
    ) -> Box<dyn ze_platform::Cursor> {
        todo!()
    }

    fn set_cursor(&self, cursor: Option<&dyn ze_platform::Cursor>) {
        todo!()
    }

    fn mouse_position(&self) -> ze_core::maths::Vec2i32 {
        todo!()
    }

    fn monitor_count(&self) -> usize {
        todo!()
    }

    fn monitor(&self, index: usize) -> ze_platform::Monitor {
        todo!()
    }
}
