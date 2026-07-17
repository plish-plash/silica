use std::{cell::RefCell, rc::Rc};

use crate::{render::GuiRenderer, *};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    #[default]
    Normal,
    Confirm,
    Delete,
    Flat,
    Tab,
    Checkbox,
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
    pub fn handle_input(&mut self, input: &GuiInput, hotkey: Option<Hotkey>, rect: Rect) -> ButtonStateInput {
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
        let clicked = *self == ButtonState::Press && (hotkey_pressed || (input.clicked && pointer_over));
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
    Custom(Rc<dyn Fn(&mut EventExecutor, &Area, &mut bool)>),
}

#[must_use]
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
    pub fn parent(mut self, parent: impl Into<NodeId>) -> Self {
        self.node = self.node.parent(parent);
        self
    }
    pub fn child(mut self, child: impl Into<NodeId>) -> Self {
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
    pub fn build<F>(self, gui: &mut Gui, on_clicked: F) -> WidgetId<Button>
    where
        F: Fn(EventContext) + 'static,
    {
        let mut button = Button::new(self.button_style, on_clicked);
        button.set_enabled(self.enabled);
        button.hotkey = self.hotkey;
        self.node.build_widget(gui, button)
    }
    pub fn build_toggle<F>(self, gui: &mut Gui, on_clicked: F) -> WidgetId<Button>
    where
        F: Fn(EventContext, bool) + 'static,
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
            node: NodeBuilder::new().style(Button::style()),
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
    fn style() -> Style {
        Style {
            min_size: Self::MIN_SIZE,
            cross_align: Align::Center,
            ..Default::default()
        }
    }
    fn create_label(gui: &mut Gui, text: &str) -> WidgetId<Label> {
        LabelBuilder::new(text)
            .style(Style {
                grow: true,
                margin: SideOffsets::new(0, 6, 0, 6),
                ..Default::default()
            })
            .font_size(Self::LABEL_FONT_SIZE)
            .align(TextAlign::Center)
            .build(gui)
    }

    pub fn new<F>(button_style: ButtonStyle, on_clicked: F) -> Self
    where
        F: Fn(EventContext) + 'static,
    {
        Button {
            button_style,
            state: ButtonState::Normal,
            hotkey: None,
            toggled: false,
            on_clicked: ButtonEvent::Normal(EventFn::new(on_clicked)),
        }
    }
    pub fn new_toggle<F>(button_style: ButtonStyle, toggled: bool, on_clicked: F) -> Self
    where
        F: Fn(EventContext, bool) + 'static,
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
    pub fn new_custom<F>(button_style: ButtonStyle, on_clicked: F) -> Self
    where
        F: Fn(&mut EventExecutor, &Area, &mut bool) + 'static,
    {
        Button {
            button_style,
            state: ButtonState::Normal,
            hotkey: None,
            toggled: false,
            on_clicked: ButtonEvent::Custom(Rc::new(on_clicked)),
        }
    }
    pub fn create<F>(gui: &mut Gui, label: &str, on_clicked: F) -> WidgetId<Self>
    where
        F: Fn(EventContext) + 'static,
    {
        ButtonBuilder::new().label(gui, label).build(gui, on_clicked)
    }
    pub fn create_toggle<F>(gui: &mut Gui, label: &str, toggled: bool, on_clicked: F) -> WidgetId<Self>
    where
        F: Fn(EventContext, bool) + 'static,
    {
        ButtonBuilder::new()
            .label(gui, label)
            .toggled(toggled)
            .build_toggle(gui, on_clicked)
    }
    pub fn create_checkbox<F>(gui: &mut Gui, toggled: bool, on_clicked: F) -> WidgetId<Self>
    where
        F: Fn(EventContext, bool) + 'static,
    {
        ButtonBuilder::new()
            .modify_style(|style| style.min_size = Size::splat(Self::MIN_SIZE.height))
            .button_style(ButtonStyle::Checkbox)
            .toggled(toggled)
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
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, area: &Area) -> InputAction {
        let state_input = self.state.handle_input(input, self.hotkey, area.content_rect);
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
                            executor.queue(group.deselect_others.clone(), Some(Box::new((group.clone(), *index))));
                            Some(*index)
                        } else {
                            None
                        };
                        executor.queue(group.on_selected.clone(), Some(Box::new(param)));
                    }
                }
                ButtonEvent::Custom(func) => func(executor, area, &mut self.toggled),
            }
        }
        state_input.action
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        renderer
            .theme()
            .draw_button(renderer, area.content_rect, self.button_style, self.toggled, self.state);
    }
}
impl WidgetId<Button> {
    pub fn enabled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self).map(|button| button.enabled()).unwrap_or(true)
    }
    pub fn set_enabled(&self, gui: &mut Gui, enabled: bool) {
        if let Some(button) = gui.get_widget_mut(*self) {
            button.set_enabled(enabled);
        }
    }
    pub fn toggled(&self, gui: &Gui) -> bool {
        gui.get_widget(*self).map(|button| button.toggled()).unwrap_or(false)
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
    pub fn new<F>(allow_deselect: bool, on_selected: F) -> Rc<Self>
    where
        F: Fn(EventContext, Option<usize>) + 'static,
    {
        let deselect_others = EventFn::new_param(|mut context, (group, index): (Rc<ExclusiveGroup>, usize)| {
            for (other_index, other_button) in group.buttons.borrow().iter().enumerate() {
                if other_index != index {
                    other_button.set_toggled(context.gui(), false);
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
pub struct TabsBuilder {
    parent: Option<NodeId>,
    group: Rc<ExclusiveGroup>,
    tabs: NodeId,
    content: Option<NodeId>,
}

impl TabsBuilder {
    pub fn new(gui: &mut Gui, group: Rc<ExclusiveGroup>) -> Self {
        let tabs = gui.create_node(Style {
            main_align: Align::Start,
            gap: 4,
            ..Default::default()
        });
        TabsBuilder {
            parent: None,
            group,
            tabs,
            content: None,
        }
    }
    pub fn parent(mut self, parent: impl Into<NodeId>) -> Self {
        self.parent = Some(parent.into());
        self
    }
    pub fn tab(self, gui: &mut Gui, label: &str, active: bool) -> Self {
        ButtonBuilder::new()
            .parent(self.tabs)
            .button_style(ButtonStyle::Tab)
            .label(gui, label)
            .toggled(active)
            .build_exclusive(gui, &self.group);
        self
    }
    pub fn tabs<'a, I>(mut self, gui: &mut Gui, labels: I, active_index: usize) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        for (index, label) in labels.into_iter().enumerate() {
            self = self.tab(gui, label, index == active_index);
        }
        self
    }
    pub fn content(mut self, content: impl Into<NodeId>) -> Self {
        self.content = Some(content.into());
        self
    }
    pub fn build(self, gui: &mut Gui) -> NodeId {
        let container = gui.create_node(Style {
            direction: Direction::ColumnReverse,
            gap: -1,
            ..Default::default()
        });
        if let Some(content) = self.content {
            gui.modify_style(content, |style| {
                style.border = SideOffsets::new_all_same(1);
                style.border_color = Some(Color::Border);
            });
            gui.add_child(container, content);
        }
        gui.add_child(container, self.tabs);
        if let Some(parent) = self.parent {
            gui.add_child(parent, container);
        }
        container
    }
}

struct PopupMenu(EventFn);

impl Widget for PopupMenu {
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, _area: &Area) -> InputAction {
        if !input.blocked && input.clicked {
            executor.queue(self.0.clone(), None);
            InputAction::Pass
        } else {
            InputAction::Grab
        }
    }
    fn draw(&mut self, _renderer: &mut GuiRenderer, _area: &Area) {}
}

pub struct DropdownButton<T> {
    button: Button,
    value: T,
    on_selected: EventFn,
    options: Vec<T>,
    option_labels: Rc<Vec<String>>,
    label: WidgetId<Label>,
    menu: Option<NodeId>,
}

impl<T: Clone + 'static> DropdownButton<T> {
    fn create_label(gui: &mut Gui, text: &str) -> WidgetId<Label> {
        LabelBuilder::new(text)
            .style(Style {
                grow: true,
                margin: SideOffsets::new(0, 6, 0, 6),
                ..Default::default()
            })
            .font_size(Button::LABEL_FONT_SIZE)
            .build(gui)
    }
    fn create_menu(gui: &mut Gui, dropdown: WidgetId<Self>) -> WidgetId<PopupMenu> {
        let mut builder = NodeBuilder::new().modify_style(|style| {
            style.direction = Direction::Column;
            style.border = SideOffsets::new_all_same(1);
        });
        let labels = gui.get_widget(dropdown).unwrap().option_labels.clone();
        for (index, label) in labels.iter().enumerate() {
            builder = builder.child(
                ButtonBuilder::new()
                    .button_style(ButtonStyle::Flat)
                    .label(gui, label)
                    .build(gui, move |mut context| {
                        let gui = context.gui();
                        dropdown.hide_menu(gui);
                        let (event, param) = dropdown.set_value_event(gui, index);
                        context.executor().queue_node(dropdown, event, Some(Box::new(param)));
                    }),
            );
        }
        builder.build_widget(
            gui,
            PopupMenu(EventFn::new(move |mut context| dropdown.hide_menu(context.gui()))),
        )
    }

    pub fn create<F>(
        gui: &mut Gui,
        options: Vec<T>,
        labels: Vec<String>,
        selected_index: usize,
        on_selected: F,
    ) -> WidgetId<Self>
    where
        F: Fn(EventContext, T) + 'static,
    {
        let show_fn = EventFn::new_param(move |mut context, rect: Rect| {
            let dropdown = context.widget_id();
            let gui = context.gui();
            let menu = Self::create_menu(gui, dropdown);
            gui.add_popup(menu, Point::new(rect.min_x(), rect.max_y()), true);
            context.widget::<Self>().menu = Some(menu.into());
        });

        let label = Self::create_label(gui, &labels[selected_index]);
        let arrow = gui.create_node(Style {
            min_size: Size::splat(32),
            ..Default::default()
        }); // TODO
        let button = Button::new_custom(ButtonStyle::Normal, move |executor, area, toggled| {
            *toggled = true;
            executor.queue(show_fn.clone(), Some(Box::new(area.content_rect)));
        });
        let value = options[selected_index].clone();
        let dropdown = DropdownButton {
            button,
            value,
            on_selected: EventFn::new_param(on_selected),
            options,
            option_labels: Rc::new(labels),
            label,
            menu: None,
        };
        NodeBuilder::new()
            .style(Button::style())
            .child(label)
            .child(Separator::create(gui))
            .child(arrow)
            .build_widget(gui, dropdown)
    }
}
impl<T: 'static> Widget for DropdownButton<T> {
    fn input(&mut self, input: &GuiInput, executor: &mut EventExecutor, area: &Area) -> InputAction {
        self.button.input(input, executor, area)
    }
    fn draw(&mut self, renderer: &mut GuiRenderer, area: &Area) {
        self.button.draw(renderer, area);
    }
}
impl<T: Clone + 'static> WidgetId<DropdownButton<T>> {
    fn hide_menu(&self, gui: &mut Gui) {
        let dropdown = gui.get_widget_mut(*self).unwrap();
        dropdown.button.set_toggled(false);
        if let Some(menu) = dropdown.menu.take() {
            gui.remove_popup(menu);
            gui.delete(menu);
        }
    }
    fn set_value_event(&self, gui: &mut Gui, index: usize) -> (EventFn, T) {
        let dropdown = gui.get_widget_mut(*self).unwrap();
        let value = dropdown.options[index].clone();
        dropdown.value = value.clone();
        let label = dropdown.label;
        let option_labels = dropdown.option_labels.clone();
        let event = dropdown.on_selected.clone();
        label.set_text(gui, &option_labels[index]);
        (event, value)
    }
    pub fn value(&self, gui: &Gui) -> T {
        gui.get_widget(*self).unwrap().value.clone()
    }
}
