use silica_gui::*;
use silica_wgpu::{AdapterFeatures, Context};
use silica_window::{Window, run_gui_app};

fn main() {
    let context = Context::init(AdapterFeatures::default());
    run_gui_app(
        Window::default_attributes().with_title("Text Example"),
        context,
        "theme/light_theme",
        |theme| {
            let mut gui = Gui::new(theme);
            let label = Label::create(&mut gui, include_str!("ipsum.txt"));
            gui.set_root(label);
            gui.set_style(
                label,
                Style {
                    padding: SideOffsets::new_all_same(8),
                    ..Default::default()
                },
            );
            gui
        },
    )
    .unwrap();
}
