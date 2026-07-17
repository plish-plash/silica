use silica_gui::*;
use silica_window::{Window, run_gui_app};

#[derive(Clone)]
enum DropdownOptions {
    One,
    Two,
    Three,
    Four,
}

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = LabelBuilder::new("Hello, World!").font_size(24.0).build(gui);
    NodeBuilder::new()
        .modify_style(|style| {
            style.direction = Direction::Column;
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
                        .build(gui, move |mut context| {
                            label.set_text(context.gui(), "Pressed Normal Button");
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .label(gui, "Toggle Button")
                        .build_toggle(gui, move |mut context, toggled| {
                            label.set_text(
                                context.gui(),
                                &format!("Toggle Button {}", if toggled { "On" } else { "Off" }),
                            );
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .button_style(ButtonStyle::Confirm)
                        .label(gui, "Confirm Button")
                        .build(gui, move |mut context| {
                            label.set_text(context.gui(), "Pressed Confirm Button");
                        }),
                )
                .child(
                    ButtonBuilder::new()
                        .modify_style(|style| style.grow = true)
                        .button_style(ButtonStyle::Delete)
                        .label(gui, "Delete Button")
                        .build(gui, move |mut context| {
                            label.set_text(context.gui(), "Pressed Delete Button");
                        }),
                )
                .build(gui),
        )
        .child(DropdownButton::create(
            gui,
            vec![
                DropdownOptions::One,
                DropdownOptions::Two,
                DropdownOptions::Three,
                DropdownOptions::Four,
            ],
            vec![
                "One".to_string(),
                "Two".to_string(),
                "Three".to_string(),
                "Four".to_string(),
            ],
            0,
            |mut context, option| {},
        ))
        .child({
            let group = ExclusiveGroup::new(false, move |mut context, index| {
                label.set_text(
                    context.gui(),
                    &format!("Selected Tab {}", index.map(|i| i + 1).unwrap_or_default()),
                );
            });
            TabsBuilder::new(gui, group)
                .tabs(gui, ["One", "Two", "Three", "Four"], 1)
                .content(
                    NodeBuilder::new()
                        .modify_style(|style| {
                            style.padding = SideOffsets::new_all_same(8);
                            style.gap = 8;
                        })
                        .child(Button::create_checkbox(gui, false, move |mut context, toggled| {
                            label.set_text(
                                context.gui(),
                                &format!("Checkbox {}", if toggled { "On" } else { "Off" }),
                            );
                        }))
                        .child({
                            let slider = Slider::create(gui, false, move |mut context, value| {
                                label.set_text(context.gui(), &format!("Moved Slider {}", value));
                            });
                            gui.modify_style(slider, |style| style.grow = true);
                            slider
                        })
                        .build(gui),
                )
                .build(gui)
        })
        .build(gui)
}

fn main() {
    let window_attributes = Window::default_attributes().with_title("Gallery Example");
    run_gui_app(window_attributes, "theme/light_theme", build_gui);
}
