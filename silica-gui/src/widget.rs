use std::{cell::RefCell, rc::Rc};

use glyphon::{
    cosmic_text::Align, Attrs, Buffer, FontSystem, Metrics, Shaping, TextArea, TextBounds,
};
use silica_wgpu::Uv;
use taffy::{AlignItems, AvailableSpace, Dimension, NodeId, Size, Style};

use crate::{
    gui, layout,
    theme::{Theme, ThemeColor},
    EventExecutor, EventFn, Gui, GuiBatcher, GuiInput, Hotkey, InputAction, Quad, Rect,
    SideOffsets, Widget, WidgetBuilder, WidgetId,
};

pub struct VisibleProperties {
    pub layout: Style,
    pub visible: bool,
    pub separate_layer: bool,
    pub background: Option<ThemeColor>,
}

impl Default for VisibleProperties {
    fn default() -> Self {
        Self {
            layout: Style::default(),
            visible: true,
            separate_layer: false,
            background: None,
        }
    }
}

pub struct Visible {
    pub visible: bool,
    pub separate_layer: bool,
    pub background: Option<ThemeColor>,
}

impl Visible {
    pub fn create(gui: &mut Gui, properties: VisibleProperties) -> WidgetId<Self> {
        gui.create_widget(
            properties.layout,
            Visible {
                visible: properties.visible,
                separate_layer: properties.separate_layer,
                background: properties.background,
            },
        )
    }
}
impl Widget for Visible {
    fn visible(&self) -> bool {
        self.visible
    }
    fn separate_layer(&self) -> bool {
        self.separate_layer
    }
    fn draw<'a>(
        &'a self,
        batcher: &mut GuiBatcher<'a>,
        theme: &dyn Theme,
        rect: Rect,
        padding: SideOffsets,
    ) {
        if let Some(background) = self.background {
            batcher.queue_theme_quad(Quad {
                rect: rect.outer_box(padding),
                uv: Uv::ZERO,
                color: theme.color(background),
            });
        }
    }
}
impl WidgetBuilder for Visible {
    type Properties<'a> = VisibleProperties;
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

pub struct LabelProperties<'a> {
    pub layout: Style,
    pub font_size: f32,
    pub line_height: f32,
    pub color: Option<glyphon::Color>,
    pub alignment: Option<Align>,
    pub family: glyphon::Family<'static>,
    pub stretch: glyphon::Stretch,
    pub style: glyphon::Style,
    pub weight: glyphon::Weight,
    pub text: &'a str,
}

impl LabelProperties<'_> {
    fn metrics(&self) -> Metrics {
        Metrics::relative(self.font_size, self.line_height)
    }
    fn attrs(&self) -> Attrs<'static> {
        Attrs {
            color_opt: self.color,
            family: self.family,
            stretch: self.stretch,
            style: self.style,
            weight: self.weight,
            ..Attrs::new()
        }
    }
}
impl Default for LabelProperties<'_> {
    fn default() -> Self {
        LabelProperties {
            layout: Style::default(),
            font_size: 18.0,
            line_height: 1.0,
            color: None,
            alignment: None,
            family: glyphon::Family::SansSerif,
            stretch: glyphon::Stretch::Normal,
            style: glyphon::Style::Normal,
            weight: glyphon::Weight::NORMAL,
            text: "",
        }
    }
}

pub struct Label(Buffer, Attrs<'static>, Option<Align>);

impl Label {
    pub fn new(
        font_system: &mut FontSystem,
        metrics: Metrics,
        attrs: Attrs<'static>,
        alignment: Option<Align>,
        text: &str,
    ) -> Self {
        let mut buffer = Buffer::new(font_system, metrics);
        if !text.is_empty() {
            buffer.set_rich_text(
                font_system,
                [(text, attrs.clone())],
                &attrs,
                Shaping::Basic,
                alignment,
            );
        }
        Label(buffer, attrs, alignment)
    }
    #[must_use]
    pub fn create(gui: &mut Gui, properties: LabelProperties) -> WidgetId<Self> {
        let widget = Label::new(
            gui.font_system(),
            properties.metrics(),
            properties.attrs(),
            properties.alignment,
            properties.text,
        );
        gui.create_widget(properties.layout, widget)
    }
    pub fn buffer(&self) -> &Buffer {
        &self.0
    }
    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.0.set_rich_text(
            font_system,
            [(text, self.1.clone())],
            &self.1,
            Shaping::Basic,
            self.2,
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
        self.0.set_size(font_system, width_constraint, None);
        let (width, total_lines) = self
            .0
            .layout_runs()
            .fold((0.0, 0usize), |(width, total_lines), run| {
                (run.line_w.max(width), total_lines + 1)
            });
        let height = total_lines as f32 * self.0.metrics().line_height;
        Size { width, height }
    }
    fn layout(&mut self, font_system: &mut FontSystem, rect: Rect) {
        self.0.set_size(font_system, Some(rect.width()), None);
    }
    fn draw<'a>(
        &'a self,
        batcher: &mut GuiBatcher<'a>,
        theme: &dyn Theme,
        rect: Rect,
        _padding: SideOffsets,
    ) {
        batcher.queue_text(TextArea {
            buffer: &self.0,
            left: rect.min.x,
            top: rect.min.y,
            scale: 1.0,
            bounds: TextBounds::default(),
            default_color: glyphon::Color(theme.color(ThemeColor::Text).to_u32()),
            custom_glyphs: &[],
        });
    }
}
impl WidgetBuilder for Label {
    type Properties<'a> = LabelProperties<'a>;
}
impl WidgetId<Label> {
    pub fn set_text(&self, gui: &mut Gui, text: &str) {
        let Some((label, font_system)) = gui.get_widget_and_font_system(*self) else {
            return;
        };
        label.set_text(font_system, text);
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

pub struct ButtonStateAction {
    pub input_action: InputAction,
    pub changed: bool,
    pub clicked: bool,
}

impl ButtonState {
    pub fn handle_input(
        &mut self,
        input: &GuiInput,
        hotkey: Option<Hotkey>,
        rect: Rect,
    ) -> ButtonStateAction {
        let pointer_over = !input.blocked && rect.contains(input.pointer);
        let input_action = if pointer_over {
            InputAction::Block
        } else {
            InputAction::Pass
        };
        if *self == ButtonState::Disable {
            return ButtonStateAction {
                input_action,
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
            return ButtonStateAction {
                input_action: InputAction::Pass,
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
        ButtonStateAction {
            input_action,
            changed,
            clicked,
        }
    }
}

pub struct ButtonProperties<'a> {
    pub layout: Style,
    pub theme: ButtonTheme,
    pub enabled: bool,
    pub label: Option<&'a str>,
}

impl Default for ButtonProperties<'_> {
    fn default() -> Self {
        Self {
            layout: Default::default(),
            theme: ButtonTheme::Normal,
            enabled: true,
            label: None,
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

pub struct Button {
    theme: ButtonTheme,
    state: ButtonState,
    toggled: bool,
    on_clicked: ButtonEvent,
}

impl Button {
    const LABEL_FONT_SIZE: f32 = 20.0;
    const MIN_SIZE: Size<Dimension> = Size::from_lengths(128.0, 32.0);

    pub fn new<C, F>(theme: ButtonTheme, on_clicked: F) -> Self
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        Button {
            theme,
            state: ButtonState::Normal,
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
            toggled,
            on_clicked: ButtonEvent::Toggle(EventFn::new_param(on_clicked)),
        }
    }
    fn new_exclusive(theme: ButtonTheme, state: bool, group: &Rc<ExclusiveGroup>) -> Self {
        Button {
            theme,
            state: ButtonState::Normal,
            toggled: state,
            on_clicked: ButtonEvent::Exclusive(group.clone(), group.next_index()),
        }
    }
    fn create_label(gui: &mut Gui, button: WidgetId<Button>, text: &str) {
        let margin = taffy::Rect::new(4.0, 4.0, 0.0, 0.0).map(taffy::LengthPercentageAuto::length);
        let label = gui! { Label(text, font_size: Self::LABEL_FONT_SIZE, alignment: Some(Align::Center), layout: layout!(flex_grow: 1.0, margin)) };
        gui.add_child(button, label);
    }
    #[must_use]
    pub fn create<C, F>(
        gui: &mut Gui,
        properties: ButtonProperties,
        on_clicked: F,
    ) -> WidgetId<Self>
    where
        C: 'static,
        F: Fn(&mut C) + 'static,
    {
        let mut button = Self::new(properties.theme, on_clicked);
        button.set_enabled(properties.enabled);
        let button = gui.create_widget(
            Style {
                min_size: Self::MIN_SIZE,
                align_items: Some(AlignItems::Center),
                ..properties.layout
            },
            button,
        );
        if let Some(label) = properties.label {
            Self::create_label(gui, button, label);
        }
        button
    }
    pub fn create_exclusive(
        gui: &mut Gui,
        properties: ToggleButtonProperties,
        group: &Rc<ExclusiveGroup>,
    ) -> WidgetId<Self> {
        let button = gui.create_widget(
            Style {
                min_size: Size {
                    width: Dimension::auto(),
                    height: Self::MIN_SIZE.height,
                },
                align_items: Some(AlignItems::Center),
                ..properties.layout
            },
            Self::new_exclusive(properties.theme, properties.toggled, group),
        );
        if let Some(label) = properties.label {
            Self::create_label(gui, button, label);
        }
        group.buttons.borrow_mut().push(button);
        button
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            self.state = ButtonState::Normal;
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
        let action = self.state.handle_input(input, None, rect);
        if action.changed {
            executor.mark_draw_dirty();
        }
        if action.clicked {
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
        action.input_action
    }
    fn draw<'a>(
        &'a self,
        batcher: &mut GuiBatcher<'a>,
        theme: &dyn Theme,
        rect: Rect,
        _padding: SideOffsets,
    ) {
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
        theme.draw_button(batcher, rect, button_theme, self.state);
    }
}
impl WidgetBuilder for Button {
    type Properties<'a> = ButtonProperties<'a>;
}
impl WidgetId<Button> {
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
            .unwrap_or_default()
    }
    pub fn set_toggled(&self, gui: &mut Gui, toggled: bool) {
        let Some(button) = gui.get_widget_mut(*self) else {
            return;
        };
        button.set_toggled(toggled);
        gui.mark_draw_dirty();
    }
}

#[derive(Default)]
pub struct ToggleButtonProperties<'a> {
    pub layout: Style,
    pub theme: ButtonTheme,
    pub toggled: bool,
    pub label: Option<&'a str>,
}

pub struct ToggleButton;

impl ToggleButton {
    #[must_use]
    pub fn create<C, F>(
        gui: &mut Gui,
        properties: ToggleButtonProperties,
        on_clicked: F,
    ) -> WidgetId<Button>
    where
        C: 'static,
        F: Fn(&mut C, bool) + 'static,
    {
        let button = gui.create_widget(
            Style {
                min_size: Button::MIN_SIZE,
                align_items: Some(AlignItems::Center),
                ..properties.layout
            },
            Button::new_toggle(properties.theme, properties.toggled, on_clicked),
        );
        if let Some(label) = properties.label {
            Button::create_label(gui, button, label);
        }
        button
    }
}
impl WidgetBuilder for ToggleButton {
    type Properties<'a> = ToggleButtonProperties<'a>;
}

pub struct ExclusiveGroup {
    allow_deselect: bool,
    deselect_others: EventFn,
    on_selected: EventFn,
    buttons: RefCell<Vec<WidgetId<Button>>>,
}

impl ExclusiveGroup {
    fn next_index(&self) -> usize {
        self.buttons.borrow().len()
    }
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
    #[must_use]
    pub fn create_container<I>(
        self: &Rc<Self>,
        gui: &mut Gui,
        container_layout: Style,
        button_layout: Style,
        tabs: bool,
        selected: Option<usize>,
        labels: I,
    ) -> NodeId
    where
        I: IntoIterator<Item: AsRef<str>>,
    {
        let theme = if tabs {
            ButtonTheme::Tab
        } else {
            ButtonTheme::Normal
        };
        let container = gui.create_container(container_layout);
        for (index, label) in labels.into_iter().enumerate() {
            let button = Button::create_exclusive(
                gui,
                ToggleButtonProperties {
                    layout: button_layout.clone(),
                    theme,
                    toggled: selected == Some(index),
                    label: Some(label.as_ref()),
                },
                self,
            );
            gui.add_child(container, button);
        }
        container
    }
}
