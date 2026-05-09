use silica_gui::*;
use silica_window::{EventLoop, EventLoopExt, Window, run_gui_app};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let context = event_loop.create_context_default_features();
    let window_attributes = Window::default_attributes().with_title("Text Example");
    run_gui_app(event_loop, context, window_attributes, "theme/light_theme", |theme| {
        let mut gui = Gui::new(theme);
        let label = Label::create(&mut gui, include_str!("ipsum.txt"));
        gui.modify_style(label, |style| {
            style.padding = SideOffsets::new_all_same(8);
        });
        let scroll = ScrollAreaBuilder::new(&mut gui, Style::default())
            .child(label)
            .vertical_scroll(&mut gui)
            .build(&mut gui);
        gui.set_root(scroll);
        gui
    })
    .unwrap();
}
