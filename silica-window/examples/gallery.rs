use std::sync::Arc;

use silica_gui::*;

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = gui! {
        Label(font_size: 24.0, text: "Hello, World!")
    };
    let root = gui! {
        Node(layout: layout!(padding: Rect::length(16.0), gap: Size::length(16.0), align_items: Some(AlignItems::Stretch), flex_direction: FlexDirection::Column)) {
            label,
            Node(layout: layout!(gap: Size::length(16.0))) {
                Button(label: Some("Normal Button"), layout: layout!(flex_grow: 1.0)) |gui| {
                    label.set_text(gui, "Pressed Normal Button");
                },
                ToggleButton(label: Some("Toggle Button"), layout: layout!(flex_grow: 1.0)) |gui, toggled| {
                    label.set_text(gui, &format!("Toggle Button {}", if toggled { "On" } else { "Off" }));
                },
                Button(label: Some("Confirm Button"), theme: ButtonTheme::Confirm, layout: layout!(flex_grow: 1.0)) |gui| {
                    label.set_text(gui, "Pressed Confirm Button");
                },
                Button(label: Some("Delete Button"), theme: ButtonTheme::Delete, layout: layout!(flex_grow: 1.0)) |gui| {
                    label.set_text(gui, "Pressed Delete Button");
                },
            },
        }
    };

    // TODO allow ExclusiveGroup in gui! macro
    let tabs = ExclusiveGroup::new(false, move |gui, index| {
        label.set_text(
            gui,
            &format!("Selected Tab {}", index.map(|i| i + 1).unwrap_or_default()),
        );
    })
    .create_container(
        gui,
        layout!(gap: Size::length(4.0)),
        layout!(),
        true,
        Some(1),
        ["One", "Two", "Three", "Four"],
    );
    gui.add_child(root, tabs);

    root
}

fn main() {
    let font =
        glyphon::fontdb::Source::Binary(Arc::new(include_bytes!("OpenSans-Regular.ttf").to_vec()));
    let font_system = glyphon::FontSystem::new_with_fonts([font]);

    let mut gui = Gui::new(font_system);
    let root = build_gui(&mut gui);
    gui.set_root(root);

    let theme_loader = theme::StandardThemeLoader::new(include_bytes!("theme.data"));
    silica_window::run_app(gui, theme_loader).unwrap();
}
