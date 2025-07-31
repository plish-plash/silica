use silica_gui::*;

fn main() {
    let mut gui = Gui::new(FontSystem::with_system_fonts());
    let label = Label::create(&mut gui, include_str!("ipsum.txt"));
    gui.set_root(label);
    gui.set_style(
        label,
        Style {
            padding: SideOffsets::new_all_same(8),
            ..Default::default()
        },
    );
    silica_window::run_gui_app(gui, include_bytes!("theme.data")).unwrap();
}
