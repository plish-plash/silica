use silica_gui::*;
use silica_window::{Window, run_gui_app};

fn build_gui(gui: &mut Gui) -> NodeId {
    let label = Label::create(gui, include_str!("ipsum.txt"));
    gui.modify_style(label, |style| {
        style.padding = SideOffsets::new_all_same(8);
    });
    let scroll = ScrollAreaBuilder::new(gui, Style::default())
        .child(label)
        .vertical_scroll(gui)
        .build(gui);
    scroll
}

fn main() {
    let window_attributes = Window::default_attributes().with_title("Text Example");
    run_gui_app(window_attributes, "theme/light_theme", build_gui);
}
