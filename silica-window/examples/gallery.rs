use silica_gui::*;
use silica_wgpu::{AdapterFeatures, Context};
use silica_window::{Window, run_gui_app};

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = LabelBuilder::new("Hello, World!").font_size(24.0).build(gui);
    NodeBuilder::new()
        .modify_style(|style| {
            style.direction = Direction::Column;
            // style.align_items = taffy::AlignItems::Stretch;
            style.padding = SideOffsets::new_all_same(16);
            style.gap = 16;
        })
        .child(label)
        .child(
            NodeBuilder::new()
                .modify_style(|style| style.gap = 16)
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .label(gui, "Normal Button")
                        .build(gui, move |gui| {
                            label.set_text(gui, "Pressed Normal Button");
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .label(gui, "Toggle Button")
                        .build_toggle(gui, move |gui, toggled| {
                            label.set_text(gui, &format!("Toggle Button {}", if toggled { "On" } else { "Off" }));
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .button_style(ButtonStyle::Confirm)
                        .label(gui, "Confirm Button")
                        .build(gui, move |gui| {
                            label.set_text(gui, "Pressed Confirm Button");
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .button_style(ButtonStyle::Delete)
                        .label(gui, "Delete Button")
                        .build(gui, move |gui| {
                            label.set_text(gui, "Pressed Delete Button");
                        }),
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
            TabsBuilder::new(gui, group)
                .tabs(gui, ["One", "Two", "Three", "Four"], 1)
                .content({
                    let content = gui.create_node(Style {
                        padding: SideOffsets::new_all_same(8),
                        ..Default::default()
                    });
                    let slider = Slider::new(false, move |gui, value| {
                        label.set_text(gui, &format!("Moved Slider {}", value));
                    });
                    let widget = gui.create_widget(
                        Style {
                            background_color: Some(Color::Gutter),
                            min_size: Size::splat(32),
                            grow: true,
                            ..Default::default()
                        },
                        slider,
                    );
                    gui.add_child(content, widget);
                    content
                })
                .build(gui)
        })
        .build(gui)
}

fn main() {
    let context = Context::init(AdapterFeatures::default());
    run_gui_app(
        Window::default_attributes().with_title("Gallery Example"),
        context,
        "theme/light_theme",
        |theme| {
            let mut gui = Gui::new(theme);
            let root = build_gui(&mut gui);
            gui.set_root(root);
            gui
        },
    )
    .unwrap();
}
