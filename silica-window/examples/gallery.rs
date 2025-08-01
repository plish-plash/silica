use silica_gui::*;
use silica_wgpu::{AdapterFeatures, Context};

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = LabelBuilder::new("Hello, World!")
        .font_size(24.0)
        .build(gui, Style::default());
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
                            label.set_text(
                                gui,
                                &format!("Toggle Button {}", if toggled { "On" } else { "Off" }),
                            );
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
            let buttons = gui.create_node(Style {
                gap: 4,
                ..Default::default()
            });
            for (index, label) in ["One", "Two", "Three", "Four"].into_iter().enumerate() {
                ButtonBuilder::new()
                    .parent(buttons)
                    .label(gui, label)
                    .hotkey(Hotkey::new(char::from_digit(index as u32 + 1, 10).unwrap()))
                    .toggled(index == 1)
                    .build_exclusive(gui, &group);
            }
            buttons
        })
        .build(gui)
}

fn main() {
    let mut gui = Gui::new(FontSystem::with_system_fonts());
    let root = build_gui(&mut gui);
    gui.set_root(root);
    let context = Context::init(AdapterFeatures::default());
    silica_window::run_gui_app(context, gui, include_bytes!("theme.data")).unwrap();
}
