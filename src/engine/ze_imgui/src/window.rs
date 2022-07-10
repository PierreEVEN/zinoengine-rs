use crate::Context;
use ze_imgui_sys::*;

pub struct WindowBuilder<'a> {
    context: &'a mut Context,
    name: &'a str,
}

impl<'a> WindowBuilder<'a> {
    pub fn new(context: &'a mut Context, name: &'a str) -> Self {
        Self { context, name }
    }

    pub fn begin(self) {
        let name = self.context.get_str_buffer().convert(self.name);
        unsafe {
            igBegin(
                name,
                std::ptr::null_mut(),
                ImGuiWindowFlags__ImGuiWindowFlags_None,
            )
        };
    }
}
