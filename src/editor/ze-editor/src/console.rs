use parking_lot::Mutex;
use std::sync::Arc;
use ze_core::logger;
use ze_core::logger::{Message, Sink};
use ze_imgui::ze_imgui_sys::ImVec2;
use ze_imgui::{Context, StyleVar, WindowFlagBits, WindowFlags};

pub struct Console {
    messages: Mutex<Vec<Message>>,
}

impl Console {
    pub fn new() -> Arc<Self> {
        let me = Arc::new(Self {
            messages: Default::default(),
        });
        logger::register_sink_weak(Arc::downgrade(&me));
        me
    }

    pub fn draw(&self, imgui: &mut Context) {
        let messages = self.messages.lock();

        imgui.push_style_var_vec2f32(StyleVar::WindowPadding, ImVec2::from(0.0));
        imgui.begin_window("Console", WindowFlags::empty());
        imgui.push_style_var_vec2f32(StyleVar::WindowPadding, ImVec2::from(5.0));
        imgui.begin_child(
            "ScrollingRegion",
            imgui.available_content_region(),
            false,
            WindowFlags::from_flag(WindowFlagBits::AlwaysUseWindowPadding),
        );
        imgui.pop_style_var(2);

        for message in messages.iter() {
            let message = format!("({}) {}", message.crate_name, message.message);
            imgui.text_wrapped(&message);
        }

        imgui.end_child();
        imgui.end_window();
    }
}

impl Sink for Console {
    fn log(&self, message: &Message) {
        self.messages.lock().push(message.clone());
    }
}
