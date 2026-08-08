use arrow_array::UInt64Array;
use comfy_table::Table;
use comfy_table::presets::UTF8_BORDERS_ONLY;
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
            Command::Stats => {
                let mut table = Table::new();

                #[rustfmt::skip]
                table
                    .load_preset(UTF8_BORDERS_ONLY)
                    .set_header(["storage layout", "value"]);

                let stats = file.stats()?;
                for (n, field) in stats.schema_ref().fields().iter().enumerate() {
                    let key = field.name();
                    let val = stats
                        .column(n)
                        .as_any()
                        .downcast_ref::<UInt64Array>()
                        .unwrap()
                        .values()[0];

                    table.add_row([key, val.to_string().as_str()]);
                }

                Some(table.to_string())
            }
        };

        Ok(msg)
    }
}
