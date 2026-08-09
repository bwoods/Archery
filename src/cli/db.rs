use reedline_repl_rs::clap::{ArgMatches, FromArgMatches, Subcommand};
use std::error::Error;
use storage::File;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compact the file (if possible)
    #[clap(visible_alias = "gc")]
    Compact,
    /// Internal statistics
    #[clap(visible_alias = "stats")]
    Layout,
    /// Page-size used for I/O
    #[clap(name = "pagesize")]
    PageSize,
    /// Size of the file on disk
    Size,
    /// Path to the file on disk
    Path,
}
pub fn commands(args: ArgMatches, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
    Ok(Command::from_arg_matches(&args)?.run(file)?)
}

impl Command {
    pub(crate) fn run(self, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
        let msg = match self {
            Command::Compact => match file.compact()? {
                false => Some("no work to do".to_string()),
                true => None,
            },
            Command::PageSize => Some(file.page_size()?.to_string()),
            Command::Layout => Some(file.stats()?.to_string()),
            Command::Size => Some(file.file_size()?.to_string()),
            Command::Path => Some(file.file_path()?.to_string()),
        };

        Ok(msg)
    }
}
