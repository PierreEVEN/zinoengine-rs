use crate::renderer::DrawContext;
use downcast_rs::{impl_downcast, Downcast};
use std::fmt::Debug;
use ze_core::maths::Vec2f32;

#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub enum Visibility {
    Hidden,
    /// Like Hidden but takes no space
    Collapsed,
    Visible,
}

/// Determine constraints for a specific region
/// Can be a box, a part of a circle etc
pub trait Constraints: Downcast {}
impl_downcast!(Constraints);

#[derive(Debug, Copy, Clone)]
pub struct BoxConstraints {
    pub min_size: Vec2f32,
    pub max_size: Vec2f32,
}

impl BoxConstraints {
    pub fn new(min_size: Vec2f32, max_size: Vec2f32) -> Self {
        Self { min_size, max_size }
    }
}

impl Constraints for BoxConstraints {}

pub trait RenderNode {
    /// Layout the widget with the given constraints from the parent
    /// Default implementation position the first child
    fn layout(&mut self, constraints: &dyn Constraints) -> Vec2f32;
    fn draw(&self, draw_context: &mut DrawContext, position: Vec2f32);
}

pub type WidgetUniqueId = usize;

/// A widget is an object composing the UI
pub trait Widget {
    /// Layout the widget with the given constraints from the parent
    /// Default implementation position the first child
    fn layout(&mut self, constraints: &dyn Constraints) -> Vec2f32;
    fn draw(&self, draw_context: &mut DrawContext, position: Vec2f32);
}

#[derive(Default)]
pub struct UiState {
    root_widget: Option<Box<dyn Widget>>,
}

impl UiState {
    pub fn set_root_widget(&mut self, widget: Option<Box<dyn Widget>>) {
        self.root_widget = widget;
    }

    pub fn draw(&mut self, _: f32, draw_context: &mut DrawContext, viewport_size: Vec2f32) {
        if let Some(root_widget) = &mut self.root_widget {
            let constraints = BoxConstraints::new(viewport_size, viewport_size);
            root_widget.layout(&constraints);
            root_widget.draw(draw_context, Vec2f32::default());
        }
    }
}

/// Macro to allow readable UI declarative syntax
#[macro_export]
macro_rules! ze_ui_decl {
    (
        $widget_type:ident
        $(.$widget_param_name:ident($widget_param_content:expr))*
        $(+ $slot_type:ident()
            $(.$slot_name:ident($slot_param:expr))*
            {
                $($slot_content:tt)+
            }
        )*
    ) => {
        $widget_type::builder()
        $(.$widget_param_name($widget_param_content))*
        $(
        .slot($slot_type::builder()
            $(.$slot_name($slot_param))*
            .content(ze_ui_decl! {
                $($slot_content)+
            })
            .build())
        )*
        .build()
    };
}

pub mod property;
pub mod renderer;
pub mod widgets;