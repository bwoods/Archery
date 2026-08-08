use reedline_repl_rs::clap::{ArgMatches, FromArgMatches, Subcommand};
use std::error::Error;
use storage::storage::File;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compact the database
    #[clap(visible_alias = "gc")]
    Compact,
    /// Shows
    Stats,
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
            Command::Stats => Some(file.stats()?.to_string()),
        };

        Ok(msg)
    }
}
