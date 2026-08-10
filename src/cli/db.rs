use reedline_repl_rs::clap::{ArgMatches, Args, FromArgMatches, Subcommand, ValueEnum};
use std::error::Error;
use storage::File;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Display disk usage statistics
    ///
    /// Displays the file system page usage for each frame in the file,
    /// as well as general overhead for metadata and tree structures.
    #[clap(alias = "du")]
    Info(Info),
    /// Reduce disk usage (if possible)
    #[clap(alias = "gc")]
    Compact,
    /// Page-size used for I/O
    #[clap(hide = true)]
    PageSize,
    /// Path to the file on disk
    Path,
    /// Size of the file on disk
    Size,
}

#[derive(Args, Debug)]
pub struct Info {
    /// Show sizes rather than page counts.
    #[arg(short, long)]
    sizes: Option<Units>,
    /// Show the sizes based on powers of 1000 (rather than 1024).
    #[arg(long, requires = "sizes")]
    si: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Units {
    /// bytes
    #[clap(name = "B")]
    B,
    /// Kilobytes
    #[clap(name = "KB")]
    KB,
    /// Megabytes
    #[clap(name = "MB")]
    MB,
    /// Gigabytes
    #[clap(name = "GB")]
    GB,
    /// Terabytes
    #[clap(name = "TB")]
    TB,
    /// Petabytes
    #[clap(name = "PB")]
    PB,
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
            Command::Info { .. } => Some(file.stats()?.to_string()),
            Command::Size => Some(file.file_size()?.to_string()),
            Command::Path => Some(file.file_path()?.to_string()),
        };

        Ok(msg)
    }
}
