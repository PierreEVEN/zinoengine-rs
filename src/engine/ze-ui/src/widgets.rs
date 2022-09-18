use crate::font::Font;
use crate::property::Property;
use crate::renderer::DrawContext;
use crate::{BoxConstraints, Constraints, LayoutContext, Widget};
use harfbuzz_rs::GlyphBuffer;
use rand::{thread_rng, Rng};
use ze_core::color::Color4f32;
use ze_core::maths::Vec2f32;

pub struct FlexSlot {
    widget: Box<dyn Widget>,
    padding: Property<EdgeInsets>,
}

impl FlexSlot {
    pub fn new(widget: Box<dyn Widget>, padding: Property<EdgeInsets>) -> Self {
        Self { widget, padding }
    }

    pub fn builder() -> FlexSlotBuilder {
        FlexSlotBuilder::default()
    }
}

#[derive(Default)]
pub struct FlexSlotBuilder {
    padding: Property<EdgeInsets>,
    content: Option<Box<dyn Widget>>,
}

impl FlexSlotBuilder {
    pub fn padding(mut self, padding: Property<EdgeInsets>) -> Self {
        self.padding = padding;
        self
    }

    pub fn content(mut self, content: Box<dyn Widget>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn build(self) -> FlexSlot {
        FlexSlot::new(
            self.content.expect("Slot content not provided"),
            self.padding,
        )
    }
}

/// Widget displaying its child on one axis (horizontal or vertical)
#[derive(Default)]
pub struct Flex<const IS_VERTICAL: bool = false> {
    children: Vec<FlexSlot>,
    children_sizes: Vec<Vec2f32>,
    size: Vec2f32,
    debug_color: Color4f32,
}

impl<const IS_VERTICAL: bool> Flex<IS_VERTICAL> {
    pub fn new(children: Vec<FlexSlot>) -> Self {
        let mut rng = thread_rng();

        let mut children_sizes = vec![];
        children_sizes.resize(children.len(), Vec2f32::default());

        Self {
            children,
            children_sizes,
            size: Default::default(),
            debug_color: Color4f32::new(rng.gen(), rng.gen(), rng.gen(), 1.0),
        }
    }

    pub fn builder() -> FlexBuilder<IS_VERTICAL> {
        FlexBuilder::default()
    }

    pub fn slot() -> FlexSlotBuilder {
        FlexSlotBuilder::default()
    }
}

impl<const IS_VERTICAL: bool> Widget for Flex<IS_VERTICAL> {
    fn layout(&mut self, context: &mut LayoutContext, constraints: &dyn Constraints) -> Vec2f32 {
        let constraints = constraints.downcast_ref::<BoxConstraints>().unwrap();
        self.size = constraints.max_size;

        let child_size = if IS_VERTICAL {
            Vec2f32::new(self.size.x, self.size.y / self.children.len() as f32)
        } else {
            Vec2f32::new(self.size.x / self.children.len() as f32, self.size.y)
        };

        for (i, child) in self.children.iter_mut().enumerate() {
            let padding = child.padding.get();
            let padding_offset =
                Vec2f32::new(padding.left + padding.right, padding.top + padding.bottom);

            let constraints =
                BoxConstraints::new(child_size - padding_offset, child_size - padding_offset);

            self.children_sizes[i] = child.widget.layout(context, &constraints);
        }

        self.size
    }

    fn draw(&mut self, draw_context: &mut DrawContext, mut position: Vec2f32) {
        draw_context.rectangle(position, self.size, self.debug_color, None);

        for (i, child) in self.children.iter_mut().enumerate() {
            let padding = child.padding.get();
            let padding = Vec2f32::new(padding.left, padding.top);

            child.widget.draw(draw_context, position + padding);
            if IS_VERTICAL {
                position.y += self.children_sizes[i].y + (padding.y * 2.0);
            } else {
                position.x += self.children_sizes[i].x + (padding.x * 2.0);
            }
        }
    }
}

#[derive(Default)]
pub struct FlexBuilder<const IS_VERTICAL: bool> {
    slots: Vec<FlexSlot>,
}

impl<const IS_VERTICAL: bool> FlexBuilder<IS_VERTICAL> {
    pub fn slot(mut self, slot: FlexSlot) -> Self {
        self.slots.push(slot);
        self
    }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(Flex::<IS_VERTICAL>::new(self.slots))
    }
}

pub type Column = Flex<false>;
pub type Row = Flex<true>;
pub type ColumnSlot = FlexSlot;
pub type RowSlot = FlexSlot;

/// Values applied to expand or shrink a rectangle
#[derive(Default, Copy, Clone)]
pub struct EdgeInsets {
    top: f32,
    left: f32,
    right: f32,
    bottom: f32,
}

impl EdgeInsets {
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            left: value,
            right: value,
            bottom: value,
        }
    }
}

#[derive(Default)]
pub struct TextBuilder {
    font: Option<Font>,
    text: Option<String>,
}

impl TextBuilder {
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(Text::new(self.font.unwrap(), self.text.unwrap()))
    }
}

pub struct Text {
    font: Font,
    _text: String,
    size: Vec2f32,
    _shaped_buffer: Option<GlyphBuffer>,
}

impl Text {
    pub fn builder() -> TextBuilder {
        TextBuilder::default()
    }

    pub fn new(font: Font, text: String) -> Self {
        Self {
            font,
            _text: text,
            size: Default::default(),
            _shaped_buffer: None,
        }
    }
}

impl Widget for Text {
    fn layout(&mut self, context: &mut LayoutContext, constraints: &dyn Constraints) -> Vec2f32 {
        let constraints = constraints.downcast_ref::<BoxConstraints>().unwrap();
        self.size = constraints.max_size;

        let _family = context.font_cache().font_family(self.font.family());

        self.size
    }

    fn draw(&mut self, draw_context: &mut DrawContext, position: Vec2f32) {
        draw_context.rectangle(
            position,
            self.size,
            Color4f32::new(0.0, 0.0, 0.0, 1.0),
            None,
        );

        //draw_context.text(position, &self.font, self.shaped_buffer.as_ref().unwrap())
    }
}
