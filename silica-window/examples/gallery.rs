use silica_gui::*;

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = Label::builder("Hello, World!").font_size(24.0).build(gui);
    Node::builder()
        .direction(taffy::FlexDirection::Column)
        .align_items(taffy::AlignItems::Stretch)
        .padding(taffy::Rect::length(16.0))
        .gap(taffy::Size::length(16.0))
        .child(label)
        .child(
            Node::builder()
                .gap(taffy::Size::length(16.0))
                .child(
                    Button::builder(move |gui| {
                        label.set_text(gui, "Pressed Normal Button");
                    })
                    .label("Normal Button")
                    .grow(1.0)
                    .build(gui),
                )
                .child(
                    Button::toggle_builder(move |gui, toggled| {
                        label.set_text(
                            gui,
                            &format!("Toggle Button {}", if toggled { "On" } else { "Off" }),
                        );
                    })
                    .label("Toggle Button")
                    .grow(1.0)
                    .build(gui),
                )
                .child(
                    Button::builder(move |gui| {
                        label.set_text(gui, "Pressed Confirm Button");
                    })
                    .label("Confirm Button")
                    .theme(ButtonTheme::Confirm)
                    .grow(1.0)
                    .build(gui),
                )
                .child(
                    Button::builder(move |gui| {
                        label.set_text(gui, "Pressed Delete Button");
                    })
                    .label("Delete Button")
                    .theme(ButtonTheme::Delete)
                    .grow(1.0)
                    .build(gui),
                )
                .build(gui),
        )
        .child({
            let group = ExclusiveGroup::new(false, move |gui, index| {
                label.set_text(
                    gui,
                    &format!("Selected Tab {}", index.map(|i| i + 1).unwrap_or_default()),
                );
            });
            let buttons = ExclusiveButtons::new_tabs(
                gui,
                group,
                Some(1),
                taffy::Style {
                    gap: taffy::Size::length(4.0),
                    ..Default::default()
                },
            );
            for (index, label) in ["One", "Two", "Three", "Four"].into_iter().enumerate() {
                buttons
                    .add_button()
                    .label(label)
                    .hotkey(Hotkey::new(char::from_digit(index as u32 + 1, 10).unwrap()))
                    .build(gui);
            }
            buttons
        })
        .build(gui)
}

fn main() {
    let mut gui = Gui::new(glyphon::FontSystem::new());
    let root = build_gui(&mut gui);
    gui.set_root(root);

    let theme_loader = theme::StandardThemeLoader::new(include_bytes!("theme.data"));
    silica_window::run_app(gui, theme_loader).unwrap();
}
