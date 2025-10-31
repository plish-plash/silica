pub use glyphon::cosmic_text::Align as TextAlign;
use glyphon::{Attrs, Buffer, Metrics, Shaping, TextArea, TextBounds, TextRenderer};

use crate::{render::GuiRenderer, *};

pub trait BufferExt {
    fn text_size(&self) -> Size;
}

impl BufferExt for Buffer {
    fn text_size(&self) -> Size {
        let (width, total_lines) = self.layout_runs().fold((0.0, 0usize), |(width, total_lines), run| {
            (run.line_w.max(width), total_lines + 1)
        });
        let height = (total_lines as f32) * self.metrics().line_height;
        Size::new(width.ceil() as i32, height.ceil() as i32)
    }
}

#[must_use]
pub struct LabelBuilder<'a> {
    node: NodeBuilder,
    font_size: f32,
    line_height: f32,
    attrs: Attrs<'static>,
    align: Option<TextAlign>,
    text: &'a str,
}

impl<'a> LabelBuilder<'a> {
    pub fn new(text: &'a str) -> Self {
        LabelBuilder {
            node: NodeBuilder::new(),
            font_size: Label::DEFAULT_FONT_SIZE,
            line_height: 1.0,
            attrs: Attrs::new(),
            align: None,
            text,
        }
    }
    pub fn style(mut self, style: Style) -> Self {
        self.node = self.node.style(style);
        self
    }
    pub fn modify_style<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Style),
    {
        self.node = self.node.modify_style(f);
        self
    }
    pub fn parent(mut self, parent: NodeId) -> Self {
        self.node = self.node.parent(parent);
        self
    }
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }
    pub fn color(mut self, color: Rgba) -> Self {
        self.attrs.color_opt = Some(glyphon::Color(color.to_u32()));
        self
    }
    pub fn font_family(mut self, family: glyphon::Family<'static>) -> Self {
        self.attrs.family = family;
        self
    }
    pub fn font_stretch(mut self, stretch: glyphon::Stretch) -> Self {
        self.attrs.stretch = stretch;
        self
    }
    pub fn font_style(mut self, style: glyphon::Style) -> Self {
        self.attrs.style = style;
        self
    }
    pub fn font_weight(mut self, weight: glyphon::Weight) -> Self {
        self.attrs.weight = weight;
        self
    }
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
        self
    }
    pub fn build_label(self, gui: &Gui) -> Label {
        Label::new(
            gui.font_system(),
            Metrics::relative(self.font_size, self.line_height),
            self.attrs,
            self.align,
            self.text,
        )
    }
    pub fn build(mut self, gui: &mut Gui) -> WidgetId<Label> {
        let node = std::mem::take(&mut self.node);
        let label = self.build_label(gui);
        node.build_widget(gui, label)
    }
}

pub struct Label {
    font_system: FontSystem,
    text_renderer: Option<TextRenderer>,
    buffer: Buffer,
    attrs: Attrs<'static>,
    align: Option<TextAlign>,
}

impl Label {
    const DEFAULT_FONT_SIZE: f32 = 18.0;
    pub fn new(
        font_system: &FontSystem,
        metrics: Metrics,
        attrs: Attrs<'static>,
        align: Option<TextAlign>,
        text: &str,
    ) -> Self {
        let mut font_system_inner = font_system.borrow_mut();
        let mut buffer = Buffer::new(&mut font_system_inner, metrics);
        if !text.is_empty() {
            buffer.set_rich_text(
                &mut font_system_inner,
                [(text, attrs.clone())],
                &attrs,
                Shaping::Advanced,
                align,
            );
        }
        Label {
            font_system: font_system.clone(),
            text_renderer: None,
            buffer,
            attrs,
            align,
        }
    }
    pub fn new_default(font_system: &FontSystem, text: &str) -> Self {
        Self::new(
            font_system,
            Metrics::relative(Self::DEFAULT_FONT_SIZE, 1.0),
            Attrs::new(),
            None,
            text,
        )
    }
    pub fn create(gui: &mut Gui, text: &str) -> WidgetId<Self> {
        let label = Self::new_default(gui.font_system(), text);
        gui.create_widget(Style::default(), label)
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_rich_text(
            &mut self.font_system.borrow_mut(),
            [(text, self.attrs.clone())],
            &self.attrs,
            Shaping::Advanced,
            self.align,
        );
    }
    pub fn set_text_and_color(&mut self, text: &str, color: Option<Rgba>) {
        self.attrs.color_opt = color.map(|color| glyphon::Color(color.to_u32()));
        self.buffer.set_rich_text(
            &mut self.font_system.borrow_mut(),
            [(text, self.attrs.clone())],
            &self.attrs,
            Shaping::Advanced,
            self.align,
        );
    }
}
impl Widget for Label {
    fn measure(&mut self, available_space: Size) -> Size {
        if available_space.is_empty() {
            return Size::zero();
        }
        let width_constraint = if available_space.width == i32::MAX {
            None
        } else {
            Some(available_space.width as f32)
        };
        let height_constraint = if available_space.height == i32::MAX {
            None
        } else {
            Some(available_space.height as f32)
        };
        self.buffer
            .set_size(&mut self.font_system.borrow_mut(), width_constraint, height_constraint);
        self.buffer.text_size()
    }
    fn layout(&mut self, area: &Area) {
        let size = area.content_rect.size.to_f32();
        self.buffer
            .set_size(&mut self.font_system.borrow_mut(), Some(size.width), Some(size.height));
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        let point = area.content_rect.origin;
        let default_color = glyphon::Color(renderer.theme().color(Color::Foreground).to_u32());
        let text_renderer = self
            .text_renderer
            .get_or_insert_with(|| renderer.create_text_renderer());
        renderer.prepare_text(
            &self.font_system,
            text_renderer,
            [TextArea {
                buffer: &self.buffer,
                left: point.x as f32,
                top: point.y as f32,
                scale: 1.0,
                bounds: TextBounds::default(),
                default_color,
                custom_glyphs: &[],
            }],
        );
        renderer.draw_text(text_renderer);
    }
}
impl WidgetId<Label> {
    pub fn set_text(&self, gui: &mut Gui, text: &str) {
        if let Some(label) = gui.get_widget_mut(*self) {
            label.set_text(text);
        }
    }
    pub fn set_text_and_color(&self, gui: &mut Gui, text: &str, color: Option<Rgba>) {
        if let Some(label) = gui.get_widget_mut(*self) {
            label.set_text_and_color(text, color);
        }
    }
}
