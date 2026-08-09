use reedline_repl_rs::clap::{ArgMatches, FromArgMatches, Subcommand};
use std::error::Error;
use storage::File;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compact the file is possible
    #[clap(visible_alias = "gc")]
    Compact,
    /// Page size used for I/O
    PageSize,
    /// Internal statistics of the b-trees stored in the file
    Statistics,
    /// Size of the file on disk
    Size,
}
pub fn commands(args: ArgMatches, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
    Ok(Command::from_arg_matches(&args)?.run(file)?)
}

impl Command {
    pub(crate) fn run(self, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
        let msg = match self {
            Command::Compact => match file.compact()? {
                true => None,
                false => Some("no work to do".to_string()),
            },
            Command::Statistics => Some(file.stats()?.to_string()),
            Command::PageSize => Some(file.page_size()?.to_string()),
            Command::Size => Some(file.file_size()?.to_string()),
        };

        Ok(msg)
    }
}
