use std::{fmt::Display, path::PathBuf};

use silica_gui::*;

#[derive(Debug)]
pub struct GameError {
    asset: Option<(PathBuf, bool)>,
    message: String,
}

impl GameError {
    pub fn from_string(message: String) -> Self {
        GameError {
            asset: None,
            message,
        }
    }
    pub fn with_read(self, asset: PathBuf) -> Self {
        GameError {
            asset: Some((asset, false)),
            ..self
        }
    }
    pub fn with_write(self, asset: PathBuf) -> Self {
        GameError {
            asset: Some((asset, true)),
            ..self
        }
    }
}
impl<T: std::error::Error> From<T> for GameError {
    fn from(value: T) -> Self {
        GameError {
            asset: None,
            message: value.to_string(),
        }
    }
}
impl Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((asset, write)) = self.asset.as_ref() {
            write!(
                f,
                "Error {} {}: {}",
                if *write { "writing" } else { "reading" },
                asset.display(),
                self.message
            )
        } else {
            f.write_str(&self.message)
        }
    }
}

pub trait ResultExt<T> {
    #[track_caller]
    fn unwrap_display(self) -> T;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Display,
{
    #[track_caller]
    fn unwrap_display(self) -> T {
        match self {
            Ok(t) => t,
            Err(e) => panic!("{}", e),
        }
    }
}

pub(crate) fn io_data_error(unsupported: bool, message: String) -> std::io::Error {
    let kind = if unsupported {
        std::io::ErrorKind::Unsupported
    } else {
        std::io::ErrorKind::InvalidData
    };
    std::io::Error::new(kind, message)
}

pub(crate) fn error_gui(error: GameError) -> Gui {
    let error = error.to_string();
    log::error!("{error}");
    let mut gui = Gui::new(crate::load_fonts().unwrap_display());
    let root = NodeBuilder::new()
        .modify_style(|style| {
            style.layout = Layout::Stack;
            style.main_align = Align::Center;
            style.cross_align = Align::Center;
        })
        .child(
            NodeBuilder::new()
                .modify_style(|style| {
                    style.direction = Direction::Column;
                    style.cross_align = Align::Center;
                    style.border = SideOffsets::new_all_same(1);
                    style.padding = SideOffsets::new(16, 8, 16, 8);
                    style.gap = 16;
                })
                .child({
                    let label = LabelBuilder::new(&error)
                        .font_size(20.0)
                        .align(TextAlign::Center)
                        .build_label(&gui);
                    NodeBuilder::new()
                        .modify_style(|style| style.max_size.width = 480)
                        .build_widget(&mut gui, label)
                })
                .child(
                    ButtonBuilder::new()
                        .label(&mut gui, "Exit")
                        .button_style(ButtonStyle::Delete)
                        .build(&mut gui, |gui: &mut Gui| gui.request_exit()),
                )
                .build(&mut gui),
        )
        .build(&mut gui);
    gui.set_root(root);
    gui
}
