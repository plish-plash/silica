use std::{cell::RefCell, rc::Rc};

use glyphon::{
    cosmic_text::Align, Attrs, Buffer, FontSystem, Metrics, Shaping, TextArea, TextBounds,
};
use silica_color::Rgba;
use silica_wgpu::Uv;
use taffy::{AlignItems, AvailableSpace, Dimension, NodeId, Size, Style};

use crate::{
    theme::{Theme, ThemeColor},
    EventExecutor, EventFn, Gui, GuiBatcher, GuiInput, Hotkey, InputAction, Quad, Rect,
    SideOffsets, Widget, WidgetId,
};

#[derive(Default)]
pub struct NodeBuilderBase {
    pub layout: Style,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

impl NodeBuilderBase {
    pub fn build(self, gui: &mut Gui) -> NodeId {
        let node = gui.create_node_with_children(self.layout, &self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, node);
        }
        node
    }
    pub fn build_widget<W: Widget>(self, gui: &mut Gui, widget: W) -> WidgetId<W> {
        self.build_widget_with_layout(gui, widget, |layout| layout)
    }
    pub fn build_widget_with_layout<W, F>(self, gui: &mut Gui, widget: W, f: F) -> WidgetId<W>
    where
        W: Widget,
        F: FnOnce(Style) -> Style,
    {
        let node = gui.create_node_with_children(f(self.layout), &self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, node);
        }
        gui.set_node_widget(node, widget)
    }
}

#[macro_export]
macro_rules! impl_node_builder {
    ($name:ty) => {
        impl $name {
            pub fn layout(mut self, layout: Style) -> Self {
                self.node.layout = layout;
                self
            }
            pub fn size(mut self, size: $crate::taffy::Size<$crate::taffy::Dimension>) -> Self {
                self.node.layout.size = size;
                self
            }
            pub fn width(mut self, width: f32) -> Self {
                self.node.layout.size.width = $crate::taffy::Dimension::length(width);
                self
            }
            pub fn height(mut self, height: f32) -> Self {
                self.node.layout.size.height = $crate::taffy::Dimension::length(height);
                self
            }
            pub fn margin(
                mut self,
                margin: $crate::taffy::Rect<$crate::taffy::LengthPercentageAuto>,
            ) -> Self {
                self.node.layout.margin = margin;
                self
            }
            pub fn padding(
                mut self,
                padding: $crate::taffy::Rect<$crate::taffy::LengthPercentage>,
            ) -> Self {
                self.node.layout.padding = padding;
                self
            }
            pub fn align_items(mut self, align_items: $crate::taffy::AlignItems) -> Self {
                self.node.layout.align_items = Some(align_items);
                self
            }
            pub fn justify_content(
                mut self,
                justify_content: $crate::taffy::JustifyContent,
            ) -> Self {
                self.node.layout.justify_content = Some(justify_content);
                self
            }
            pub fn gap(
                mut self,
                gap: $crate::taffy::Size<$crate::taffy::LengthPercentage>,
            ) -> Self {
                self.node.layout.gap = gap;
                self
            }
            pub fn grow(mut self, flex_grow: f32) -> Self {
                self.node.layout.flex_grow = flex_grow;
                self
            }
            pub fn direction(mut self, flex_direction: $crate::taffy::FlexDirection) -> Self {
                self.node.layout.flex_direction = flex_direction;
                self
            }
            pub fn parent(mut self, parent: impl Into<NodeId>) -> Self {
                self.node.parent = Some(parent.into());
                self
            }
            pub fn child(mut self, child: impl Into<NodeId>) -> Self {
                self.node.children.push(child.into());
                self
            }
            pub fn children(mut self, children: impl IntoIterator<Item = NodeId>) -> Self {
                self.node.children.extend(children);
                self
            }
        }
    };
}

#[must_use]
pub struct NodeBuilder {
    node: NodeBuilderBase,
}

impl_node_builder!(NodeBuilder);
impl NodeBuilder {
    pub fn build(self, gui: &mut Gui) -> NodeId {
        self.node.build(gui)
    }
}

pub struct Node;

impl Node {
    pub fn builder() -> NodeBuilder {
        NodeBuilder {
            node: NodeBuilderBase::default(),
        }
    }
}

#[must_use]
pub struct VisibleBuilder {
    node: NodeBuilderBase,
    visible: bool,
    separate_layer: bool,
    background: Option<ThemeColor>,
}

impl_node_builder!(VisibleBuilder);
impl VisibleBuilder {
    pub fn border(mut self, border: taffy::Rect<taffy::LengthPercentage>) -> Self {
        self.node.layout.border = border;
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn separate_layer(mut self) -> Self {
        self.separate_layer = true;
        self
    }
    pub fn background(mut self, background: ThemeColor) -> Self {
        self.background = Some(background);
        self
    }
    pub fn build(self, gui: &mut Gui) -> WidgetId<Visible> {
        self.node.build_widget(
            gui,
            Visible {
                visible: self.visible,
                separate_layer: self.separate_layer,
                background: self.background,
            },
        )
    }
}

pub struct Visible {
    visible: bool,
    separate_layer: bool,
    background: Option<ThemeColor>,
}

impl Visible {
    pub fn new() -> Self {
        Visible {
            visible: true,
            separate_layer: false,
            background: None,
        }
    }
    pub fn create(gui: &mut Gui) -> WidgetId<Self> {
        gui.create_widget(Style::DEFAULT, Self::new())
    }
    pub fn builder() -> VisibleBuilder {
        VisibleBuilder {
            node: NodeBuilderBase::default(),
            visible: true,
            separate_layer: false,
            background: None,
        }
    }
}
impl Widget for Visible {
    fn visible(&self) -> bool {
        self.visible
    }
    fn separate_layer(&self) -> bool {
        self.separate_layer
    }
    fn draw_background<'a>(
        &'a self,
        batcher: &mut GuiBatcher<'a>,
        theme: &dyn Theme,
        rect: Rect,
        border: SideOffsets,
    ) {
        if let Some(background) = self.background {
            batcher.queue_theme_quad(Quad {
                rect: rect.inner_box(border),
                uv: Uv::ZERO,
                color: theme.color(background),
            });
        }
        theme.draw_border(batcher, rect, border);
    }
    fn draw<'a>(&'a self, _batcher: &mut GuiBatcher<'a>, _theme: &dyn Theme, _content_rect: Rect) {}
}
impl WidgetId<Visible> {
    pub fn visible(&self, gui: &Gui) -> bool {
        gui.get_widget(*self)
            .map(|widget| widget.visible)
            .unwrap_or_default()
    }
    pub fn set_visible(&self, gui: &mut Gui, visible: bool) {
        let Some(widget) = gui.get_widget_mut(*self) else {
            return;
        };
        widget.visible = visible;
        gui.mark_draw_dirty();
    }
    pub fn set_background(&self, gui: &mut Gui, background: Option<ThemeColor>) {
        let Some(widget) = gui.get_widget_mut(*self) else {
            return;
        };
        widget.background = background;
        gui.mark_draw_dirty();
    }
}

#[must_use]
pub struct LabelBuilder<'a> {
    node: NodeBuilderBase,
    font_size: f32,
    line_height: f32,
    attrs: Attrs<'static>,
    align: Option<Align>,
    text: &'a str,
}

impl_node_builder!(LabelBuilder<'_>);
impl LabelBuilder<'_> {
    fn metrics(&self) -> Metrics {
        Metrics::relative(self.font_size, self.line_height)
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
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }
    pub fn family(mut self, family: glyphon::Family<'static>) -> Self {
        self.attrs.family = family;
        self
    }
    pub fn stretch(mut self, stretch: glyphon::Stretch) -> Self {
        self.attrs.stretch = stretch;
        self
    }
    pub fn style(mut self, style: glyphon::Style) -> Self {
        self.attrs.style = style;
        self
    }
    pub fn weight(mut self, weight: glyphon::Weight) -> Self {
        self.attrs.weight = weight;
        self
    }
    pub fn build(self, gui: &mut Gui) -> WidgetId<Label> {
        let widget = Label::new(
            gui.font_system(),
            self.metrics(),
            self.attrs,
            self.align,
            self.text,
        );
        self.node.build_widget(gui, widget)
    }
}

pub struct Label {
    buffer: Buffer,
    attrs: Attrs<'static>,
    align: Option<Align>,
}

impl Label {
    const DEFAULT_FONT_SIZE: f32 = 18.0;
    pub fn buffer_size(buffer: &Buffer) -> Size<f32> {
        let (width, total_lines) = buffer
            .layout_runs()
            .fold((0.0, 0usize), |(width, total_lines), run| {
                (run.line_w.max(width), total_lines + 1)
            });
        let height = total_lines as f32 * buffer.metrics().line_height;
        Size {
            width: width.ceil(),
            height: height.ceil(),
        }
    }
    pub fn new(
        font_system: &mut FontSystem,
        metrics: Metrics,
        attrs: Attrs<'static>,
        align: Option<Align>,
        text: &str,
    ) -> Self {
        let mut buffer = Buffer::new(font_system, metrics);
        if !text.is_empty() {
            buffer.set_rich_text(
                font_system,
                [(text, attrs.clone())],
                &attrs,
                Shaping::Advanced,
                align,
            );
        }
        Label {
            buffer,
            attrs,
            align,
        }
    }
    pub fn create(gui: &mut Gui, text: &str) -> WidgetId<Self> {
        let label = Self::new(
            gui.font_system(),
            Metrics::relative(Self::DEFAULT_FONT_SIZE, 1.0),
            Attrs::new(),
            None,
            text,
        );
        gui.create_widget(Style::DEFAULT, label)
    }
    pub fn builder(text: &str) -> LabelBuilder {
        LabelBuilder {
            node: NodeBuilderBase::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: 1.0,
            attrs: Attrs::new(),
            align: None,
            text,
        }
    }
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.buffer.set_rich_text(
            font_system,
            [(text, self.attrs.clone())],
            &self.attrs,
            Shaping::Advanced,
            self.align,
        );
    }
    pub fn set_text_and_color(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        color: Option<Rgba>,
    ) {
        self.attrs.color_opt = color.map(|color| glyphon::Color(color.to_u32()));
        self.buffer.set_rich_text(
            font_system,
            [(text, self.attrs.clone())],
            &self.attrs,
            Shaping::Advanced,
            self.align,
        );
    }
}
impl Widget for Label {
    fn measure(
        &mut self,
        font_system: &mut FontSystem,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        let width_constraint = known_dimensions.width.or(match available_space.width {
            AvailableSpace::MinContent => None, // TODO handle MinContent correctly
            AvailableSpace::MaxContent => None,
            AvailableSpace::Definite(width) => Some(width),
        });
        self.buffer.set_size(font_system, width_constraint, None);
        Self::buffer_size(&self.buffer)
    }
    fn layout(&mut self, font_system: &mut FontSystem, content_rect: Rect) {
        self.buffer
            .set_size(font_system, Some(content_rect.width()), None);
    }
    fn draw<'a>(&'a self, batcher: &mut GuiBatcher<'a>, theme: &dyn Theme, content_rect: Rect) {
        batcher.queue_text(TextArea {
            buffer: &self.buffer,
            left: content_rect.min.x,
            top: content_rect.min.y,
            scale: 1.0,
            bounds: TextBounds::default(),
            default_color: glyphon::Color(theme.color(ThemeColor::Text).to_u32()),
            custom_glyphs: &[],
        });
    }
}
impl WidgetId<Label> {
    pub fn set_text(&self, gui: &mut Gui, text: &str) {
        let Some((label, font_system)) = gui.get_widget_and_font_system(*self) else {
            return;
        };
        label.set_text(font_system, text);
        gui.mark_layout_dirty(*self);
    }
    pub fn set_text_and_color(&self, gui: &mut Gui, text: &str, color: Option<Rgba>) {
        let Some((label, font_system)) = gui.get_widget_and_font_system(*self) else {
            return;
        };
        label.set_text_and_color(font_system, text, color);
        gui.mark_layout_dirty(*self);
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTheme {
    #[default]
    Normal,
    Toggled,
    Confirm,
    Delete,
    Flat,
    Tab,
    TabCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hover,
    Press,
    Disable,
}

pub struct ButtonStateInput {
    pub action: InputAction,
    pub changed: bool,
    pub clicked: bool,
}

impl ButtonState {
    pub fn handle_input(
        &mut self,
        input: &GuiInput,
        hotkey: Option<Hotkey>,
        rect: Rect,
    ) -> ButtonStateInput {
        let pointer_over = !input.blocked && rect.contains(input.pointer);
        let action = if pointer_over {
            InputAction::Block
        } else {
            InputAction::Pass
        };
        if *self == ButtonState::Disable {
            return ButtonStateInput {
                action,
                changed: false,
                clicked: false,
            };
        }
        let mut changed = false;
        let hotkey_pressed = input.hotkey.is_some() && input.hotkey == hotkey;
        if !hotkey_pressed && !input.grabbed && !pointer_over {
            if *self != ButtonState::Normal {
                *self = ButtonState::Normal;
                changed = true;
            }
            return ButtonStateInput {
                action: InputAction::Pass,
                changed,
                clicked: false,
            };
        }
        let state = if hotkey_pressed || input.button_pressed {
            ButtonState::Press
        } else if pointer_over {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };
        if *self != state {
            *self = state;
            changed = true;
        }
        let clicked = *self == ButtonState::Press && (hotkey_pressed || input.clicked);
        ButtonStateInput {
            action,
            changed,
            clicked,
        }
    }
}

#[derive(Clone)]
enum ButtonEvent {
    Normal(EventFn),
    Toggle(EventFn),
    Exclusive(Rc<ExclusiveGroup>, usize),
}

impl ButtonEvent {
    fn is_toggle(&self) -> bool {
        !matches!(self, ButtonEvent::Normal(_))
    }
}

#[must_use]
pub struct ButtonBuilder<'a> {
    node: NodeBuilderBase,
    on_clicked: ButtonEvent,
    theme: ButtonTheme,
    enabled: bool,
    toggled: bool,
    hotkey: Option<Hotkey>,
    label: Option<&'a str>,
}

impl_node_builder!(ButtonBuilder<'_>);
impl ButtonBuilder<'_> {
    pub fn theme(mut self, theme: ButtonTheme) -> Self {
        self.theme = theme;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.toggled = toggled;
        self
    }
    pub fn hotkey(mut self, hotkey: Hotkey) -> Self {
        self.hotkey = Some(hotkey);
        self
    }
    pub fn label<'a>(self, label: &'a str) -> ButtonBuilder<'a> {
        ButtonBuilder {
            label: Some(label),
            ..self
        }
    }
    pub fn build(mut self, gui: &mut Gui) -> WidgetId<Button> {
        let exclusive_group = if let ButtonEvent::Exclusive(group, index) = &mut self.on_clicked {
            *index = group.buttons.borrow().len();
            Some(group.clone())
        } else {
            None
        };
        let widget = Button {
            theme: self.theme,
            state: if self.enabled {
                ButtonState::Normal
            } else {
                ButtonState::Disable
            },
            hotkey: self.hotkey,
            toggled: self.toggled && self.on_clicked.is_toggle(),
            on_clicked: self.on_clicked,
        };
        let button = self
            .node
            .build_widget_with_layout(gui, widget, Button::style);
        if let Some(text) = self.label {
            Button::add_label(gui, button, text);
        }
        if let Some(group) = exclusive_group {
            group.buttons.borrow_mut().push(button);
        }
        button
    }
}

pub struct Button {
    theme: ButtonTheme,
    state: ButtonState,
    hotkey: Option<Hotkey>,
    toggled: bool,
    on_clicked: ButtonEvent,
}

impl Button {
    const LABEL_FONT_SIZE: f32 = 20.0;
    const MIN_SIZE: Size<Dimension> = Size::from_lengths(128.0, 32.0);
    fn style(base: Style) -> Style {
        Style {
            min_size: Self::MIN_SIZE,
            align_items: Some(AlignItems::Center),
            ..base
        }
    }
    fn add_label(gui: &mut Gui, button: WidgetId<Button>, text: &str) {
        use taffy::prelude::*;
        Label::builder(text)
            .layout(Style {
                flex_grow: 1.0,
                margin: Rect::new(4.0, 4.0, 0.0, 0.0).map(length),
                ..Default::default()
            })
            .font_size(Self::LABEL_FONT_SIZE)
            .align(Align::Center)
            .parent(button)
            .build(gui);
    }

    pub fn new<C, F>(theme: ButtonTheme, on_clicked: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        Button {
            theme,
            state: ButtonState::Normal,
            hotkey: None,
            toggled: false,
            on_clicked: ButtonEvent::Normal(EventFn::new(on_clicked)),
        }
    }
    pub fn new_toggle<C, F>(theme: ButtonTheme, toggled: bool, on_clicked: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        Button {
            theme,
            state: ButtonState::Normal,
            hotkey: None,
            toggled,
            on_clicked: ButtonEvent::Toggle(EventFn::new_param(on_clicked)),
        }
    }
    pub fn create<C, F>(gui: &mut Gui, label: &str, on_clicked: F) -> WidgetId<Button>
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        let button = gui.create_widget(
            Self::style(Style::DEFAULT),
            Button::new(ButtonTheme::Normal, on_clicked),
        );
        Self::add_label(gui, button, label);
        button
    }
    pub fn create_toggle<C, F>(gui: &mut Gui, label: &str, on_clicked: F) -> WidgetId<Button>
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        let button = gui.create_widget(
            Self::style(Style::DEFAULT),
            Button::new_toggle(ButtonTheme::Normal, false, on_clicked),
        );
        Self::add_label(gui, button, label);
        button
    }
    pub fn builder<C, F>(on_clicked: F) -> ButtonBuilder<'static>
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        ButtonBuilder {
            node: NodeBuilderBase::default(),
            on_clicked: ButtonEvent::Normal(EventFn::new(on_clicked)),
            theme: ButtonTheme::Normal,
            enabled: true,
            toggled: false,
            hotkey: None,
            label: None,
        }
    }
    pub fn toggle_builder<C, F>(on_clicked: F) -> ButtonBuilder<'static>
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        ButtonBuilder {
            node: NodeBuilderBase::default(),
            on_clicked: ButtonEvent::Toggle(EventFn::new_param(on_clicked)),
            theme: ButtonTheme::Normal,
            enabled: true,
            toggled: false,
            hotkey: None,
            label: None,
        }
    }
    pub fn exclusive_builder(group: Rc<ExclusiveGroup>) -> ButtonBuilder<'static> {
        ButtonBuilder {
            node: NodeBuilderBase::default(),
            on_clicked: ButtonEvent::Exclusive(group, 0),
            theme: ButtonTheme::Normal,
            enabled: true,
            toggled: false,
            hotkey: None,
            label: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.state != ButtonState::Disable
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            if self.state == ButtonState::Disable {
                self.state = ButtonState::Normal;
            }
        } else {
            self.state = ButtonState::Disable;
        }
    }
    pub fn toggled(&self) -> bool {
        self.toggled
    }
    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }
}
impl Widget for Button {
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, rect: Rect) -> InputAction {
        let state_input = self.state.handle_input(input, self.hotkey, rect);
        if state_input.changed {
            executor.mark_draw_dirty();
        }
        if state_input.clicked {
            match &self.on_clicked {
                ButtonEvent::Normal(event) => executor.queue(event.clone(), None),
                ButtonEvent::Toggle(event) => {
                    self.toggled = !self.toggled;
                    executor.queue(event.clone(), Some(Box::new(self.toggled)));
                }
                ButtonEvent::Exclusive(group, index) => {
                    if !self.toggled || group.allow_deselect {
                        self.toggled = !self.toggled;
                        let param = if self.toggled {
                            executor.queue(
                                group.deselect_others.clone(),
                                Some(Box::new((group.clone(), *index))),
                            );
                            Some(*index)
                        } else {
                            None
                        };
                        executor.queue(group.on_selected.clone(), Some(Box::new(param)));
                    }
                }
            }
        }
        state_input.action
    }
    fn draw<'a>(&'a self, batcher: &mut GuiBatcher<'a>, theme: &dyn Theme, content_rect: Rect) {
        let button_theme = if self.on_clicked.is_toggle() {
            if self.theme == ButtonTheme::Tab {
                if self.toggled {
                    ButtonTheme::TabCurrent
                } else {
                    ButtonTheme::Tab
                }
            } else if self.toggled {
                ButtonTheme::Toggled
            } else {
                ButtonTheme::Normal
            }
        } else {
            self.theme
        };
        theme.draw_button(batcher, content_rect, button_theme, self.state);
    }
}
impl WidgetId<Button> {
    pub fn enabled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self)
            .map(|button| button.enabled())
            .unwrap_or(true)
    }
    pub fn set_enabled(&self, gui: &mut Gui, enabled: bool) {
        let Some(button) = gui.get_widget_mut(*self) else {
            return;
        };
        button.set_enabled(enabled);
        gui.mark_draw_dirty();
    }
    pub fn toggled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self)
            .map(|button| button.toggled())
            .unwrap_or(false)
    }
    pub fn set_toggled(&self, gui: &mut Gui, toggled: bool) {
        let Some(button) = gui.get_widget_mut(*self) else {
            return;
        };
        button.set_toggled(toggled);
        gui.mark_draw_dirty();
    }
}

pub struct ExclusiveGroup {
    allow_deselect: bool,
    deselect_others: EventFn,
    on_selected: EventFn,
    buttons: RefCell<Vec<WidgetId<Button>>>,
}

impl ExclusiveGroup {
    pub fn new<C, F>(allow_deselect: bool, on_selected: F) -> Rc<Self>
    where
        C: 'static,
        F: Fn(&mut C, Option<usize>) + 'static,
    {
        let deselect_others =
            EventFn::new_param(|gui, (group, index): (Rc<ExclusiveGroup>, usize)| {
                for (other_index, other_button) in group.buttons.borrow().iter().enumerate() {
                    if other_index != index {
                        other_button.set_toggled(gui, false);
                    }
                }
            });
        Rc::new(ExclusiveGroup {
            allow_deselect,
            deselect_others,
            on_selected: EventFn::new_param(on_selected),
            buttons: RefCell::new(Vec::new()),
        })
    }
}

#[must_use]
pub struct ExclusiveButtons {
    group: Rc<ExclusiveGroup>,
    selected: Option<usize>,
    node: NodeId,
    button_layout: Style,
    button_theme: ButtonTheme,
}

impl ExclusiveButtons {
    pub fn new(
        gui: &mut Gui,
        group: Rc<ExclusiveGroup>,
        selected: Option<usize>,
        layout: Style,
    ) -> Self {
        ExclusiveButtons {
            group,
            selected,
            node: gui.create_node(layout),
            button_layout: Style::DEFAULT,
            button_theme: ButtonTheme::Normal,
        }
    }
    pub fn new_tabs(
        gui: &mut Gui,
        group: Rc<ExclusiveGroup>,
        selected: Option<usize>,
        layout: Style,
    ) -> Self {
        ExclusiveButtons {
            group,
            selected,
            node: gui.create_node(layout),
            button_layout: Style::DEFAULT,
            button_theme: ButtonTheme::Tab,
        }
    }
    pub fn button_layout(mut self, button_layout: Style) -> Self {
        self.button_layout = button_layout;
        self
    }
    pub fn add_button(&self) -> ButtonBuilder {
        let index = self.group.buttons.borrow().len();
        Button::exclusive_builder(self.group.clone())
            .layout(self.button_layout.clone())
            .theme(self.button_theme)
            .toggled(self.selected == Some(index))
            .parent(self.node)
    }
}
impl From<ExclusiveButtons> for NodeId {
    fn from(value: ExclusiveButtons) -> Self {
        value.node
    }
}
