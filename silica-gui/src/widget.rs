use std::{cell::RefCell, rc::Rc};

pub use glyphon::cosmic_text::Align as TextAlign;
use glyphon::{Attrs, Buffer, Metrics, Shaping, TextArea, TextBounds, TextRenderer};

use crate::{render::GuiRenderer, *};

#[derive(Default)]
pub struct NodeBuilder {
    style: Style,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn modify_style<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Style),
    {
        f(&mut self.style);
        self
    }
    pub fn parent(mut self, parent: impl Into<NodeId>) -> Self {
        self.parent = Some(parent.into());
        self
    }
    pub fn child(mut self, child: impl Into<NodeId>) -> Self {
        self.children.push(child.into());
        self
    }
    pub fn build(self, gui: &mut Gui) -> NodeId {
        let node = gui.create_node(self.style);
        gui.set_node_children(node, self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, node);
        }
        node
    }
    pub fn build_widget<W: Widget>(self, gui: &mut Gui, widget: W) -> WidgetId<W> {
        let widget = gui.create_widget(self.style, widget);
        gui.set_node_children(widget, self.children);
        if let Some(parent) = self.parent {
            gui.add_child(parent, widget);
        }
        widget
    }
}

#[must_use]
pub struct LabelBuilder<'a> {
    font_size: f32,
    line_height: f32,
    attrs: Attrs<'static>,
    align: Option<TextAlign>,
    text: &'a str,
}

impl<'a> LabelBuilder<'a> {
    pub fn new(text: &'a str) -> Self {
        LabelBuilder {
            font_size: Label::DEFAULT_FONT_SIZE,
            line_height: 1.0,
            attrs: Attrs::new(),
            align: None,
            text,
        }
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
    pub fn build(self, gui: &Gui) -> Label {
        Label::new(
            gui.font_system(),
            Metrics::relative(self.font_size, self.line_height),
            self.attrs,
            self.align,
            self.text,
        )
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
        self.buffer.set_size(
            &mut self.font_system.borrow_mut(),
            width_constraint,
            height_constraint,
        );
        let (width, total_lines) = self
            .buffer
            .layout_runs()
            .fold((0.0, 0usize), |(width, total_lines), run| {
                (run.line_w.max(width), total_lines + 1)
            });
        let height = (total_lines as f32) * self.buffer.metrics().line_height;
        Size::new(width.ceil() as i32, height.ceil() as i32)
    }
    fn layout(&mut self, area: &Area) {
        let size = area.content_rect.size.to_f32();
        self.buffer.set_size(
            &mut self.font_system.borrow_mut(),
            Some(size.width),
            Some(size.height),
        );
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

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    #[default]
    Normal,
    Confirm,
    Delete,
    Flat,
    Tab,
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

pub struct ButtonBuilder {
    node: NodeBuilder,
    button_style: ButtonStyle,
    enabled: bool,
    toggled: bool,
    hotkey: Option<Hotkey>,
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self::default()
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
    pub fn child(mut self, child: NodeId) -> Self {
        self.node = self.node.child(child);
        self
    }
    pub fn button_style(mut self, button_style: ButtonStyle) -> Self {
        self.button_style = button_style;
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
    pub fn label(mut self, gui: &mut Gui, label: &str) -> Self {
        let label = Button::create_label(gui, label);
        self.node = self.node.child(label);
        self
    }
    pub fn build<C, F>(self, gui: &mut Gui, on_clicked: F) -> WidgetId<Button>
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        let mut button = Button::new(self.button_style, on_clicked);
        button.set_enabled(self.enabled);
        button.hotkey = self.hotkey;
        self.node.build_widget(gui, button)
    }
    pub fn build_toggle<C, F>(self, gui: &mut Gui, on_clicked: F) -> WidgetId<Button>
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        let mut button = Button::new_toggle(self.button_style, self.toggled, on_clicked);
        button.set_enabled(self.enabled);
        button.hotkey = self.hotkey;
        self.node.build_widget(gui, button)
    }
    pub fn build_exclusive(self, gui: &mut Gui, group: &Rc<ExclusiveGroup>) -> WidgetId<Button> {
        let mut button = Button::new_exclusive(self.button_style, self.toggled, group.clone());
        button.set_enabled(self.enabled);
        button.hotkey = self.hotkey;
        let widget = self.node.build_widget(gui, button);
        group.buttons.borrow_mut().push(widget);
        widget
    }
}
impl Default for ButtonBuilder {
    fn default() -> Self {
        ButtonBuilder {
            node: NodeBuilder::new().style(Button::default_style()),
            button_style: ButtonStyle::default(),
            enabled: true,
            toggled: false,
            hotkey: None,
        }
    }
}

pub struct Button {
    button_style: ButtonStyle,
    state: ButtonState,
    hotkey: Option<Hotkey>,
    toggled: bool,
    on_clicked: ButtonEvent,
}

impl Button {
    const LABEL_FONT_SIZE: f32 = 20.0;
    const MIN_SIZE: Size = Size::new(128, 32);
    fn default_style() -> Style {
        Style {
            min_size: Self::MIN_SIZE,
            align: Align::Center,
            ..Default::default()
        }
    }
    fn create_label(gui: &mut Gui, text: &str) -> WidgetId<Label> {
        let label = LabelBuilder::new(text)
            .font_size(Self::LABEL_FONT_SIZE)
            .align(TextAlign::Center)
            .build(gui);
        gui.create_widget(
            Style {
                grow: true,
                margin: SideOffsets::new(0, 4, 0, 4),
                ..Default::default()
            },
            label,
        )
    }

    pub fn new<C, F>(button_style: ButtonStyle, on_clicked: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        Button {
            button_style,
            state: ButtonState::Normal,
            hotkey: None,
            toggled: false,
            on_clicked: ButtonEvent::Normal(EventFn::new(on_clicked)),
        }
    }
    pub fn new_toggle<C, F>(button_style: ButtonStyle, toggled: bool, on_clicked: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        Button {
            button_style,
            state: ButtonState::Normal,
            hotkey: None,
            toggled,
            on_clicked: ButtonEvent::Toggle(EventFn::new_param(on_clicked)),
        }
    }
    fn new_exclusive(button_style: ButtonStyle, toggled: bool, group: Rc<ExclusiveGroup>) -> Self {
        let index = group.buttons.borrow().len();
        Button {
            button_style,
            state: ButtonState::Normal,
            hotkey: None,
            toggled,
            on_clicked: ButtonEvent::Exclusive(group, index),
        }
    }
    pub fn create<C, F>(gui: &mut Gui, label: &str, on_clicked: F) -> WidgetId<Self>
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        ButtonBuilder::new()
            .label(gui, label)
            .build(gui, on_clicked)
    }
    pub fn create_toggle<C, F>(gui: &mut Gui, label: &str, on_clicked: F) -> WidgetId<Self>
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        ButtonBuilder::new()
            .label(gui, label)
            .build_toggle(gui, on_clicked)
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
    fn input(
        &mut self,
        input: &GuiInput,
        executor: &mut EventExecutor,
        area: &Area,
    ) -> InputAction {
        let state_input = self
            .state
            .handle_input(input, self.hotkey, area.content_rect);
        if state_input.changed {
            executor.request_redraw();
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
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        renderer.theme().draw_button(
            renderer,
            area.content_rect,
            self.button_style,
            self.toggled,
            self.state,
        );
    }
}
impl WidgetId<Button> {
    pub fn enabled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self)
            .map(|button| button.enabled())
            .unwrap_or(true)
    }
    pub fn set_enabled(&self, gui: &mut Gui, enabled: bool) {
        if let Some(button) = gui.get_widget_mut(*self) {
            button.set_enabled(enabled);
        }
    }
    pub fn toggled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self)
            .map(|button| button.toggled())
            .unwrap_or(false)
    }
    pub fn set_toggled(&self, gui: &mut Gui, toggled: bool) {
        if let Some(button) = gui.get_widget_mut(*self) {
            button.set_toggled(toggled);
        }
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
