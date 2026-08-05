use comfy_table::Table;
use comfy_table::presets::UTF8_BORDERS_ONLY;
use reedline_repl_rs::clap::{ArgMatches, FromArgMatches, Subcommand};
use std::error::Error;
use storage::file::File;

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
            Command::Stats => {
                let stats = file.stats()?;
                let mut table = Table::new();

                #[rustfmt::skip]
                table
                    .load_preset(UTF8_BORDERS_ONLY)
                    .set_header(["storage layout", "value"])
                    .add_row(["tree height", stats.tree_height().to_string().as_str()])
                    .add_row(["branch pages", stats.branch_pages().to_string().as_str()])
                    .add_row(["leaf pages", stats.leaf_pages().to_string().as_str()])
                    .add_row(["metadata bytes",stats.metadata_bytes().to_string().as_str(),])
                    .add_row(["stored bytes", stats.stored_bytes().to_string().as_str()])
                    .add_row(["fragmented bytes",stats.fragmented_bytes().to_string().as_str(),])
                    .add_row(["allocated pages",stats.allocated_pages().to_string().as_str(),])
                    .add_row(["page size",stats.page_size().to_string().as_str(),]);

                Some(table.to_string())
            }
        };

        Ok(msg)
    }
}
