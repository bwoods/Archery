use crate::{Args as Arguments, Noun};
use clap::{Args, ValueEnum};
use clap_tui::{Theme, ThemePreset, Tui, TuiConfig};
use std::error::Error;
use std::time::Duration;
use storage::File;

#[derive(Args, Debug)]
pub struct UI {
    /// Explicitly set light or dark mode
    #[arg(short, long)]
    theme: Option<Mode>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Mode {
    /// light mode
    Light,
    /// dark mode
    Dark,
}

fn dark() -> Theme {
    Theme::from_preset(ThemePreset::HighContrastDark)
    // TODO: further customization
}

fn light() -> Theme {
    Theme::from_preset(ThemePreset::Light)
    // TODO: further customization
}

pub fn run<F>(file: File, ui: UI, mut cli: F) -> Result<Option<String>, Box<dyn Error>>
where
    F: FnMut(File, Noun) -> Result<Option<String>, Box<dyn Error>>,
{
    let mut config = TuiConfig::default();
    config.theme = match ui.theme {
        Some(Mode::Light) => light(),
        Some(Mode::Dark) => dark(),
        None => match termbg::theme(Duration::from_millis(250)) {
            Ok(termbg::Theme::Light) => light(),
            Ok(termbg::Theme::Dark) | Err(_) => dark(),
        },
    };

    let tui = Tui::<Arguments>::new()
        .with_config(config)
        .hide_entrypoint("script")?
        .hide_entrypoint("help")?;

    if let Some(args) = tui.run()? {
        cli(file, args.command)
    } else {
        Ok(None)
    }
}
