use arrow_array::StringArray;
use comfy_table::Table;
use comfy_table::presets::UTF8_BORDERS_ONLY;
use itertools::Itertools;
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

                let stats = file.stats()?;
                let header = stats
                    .schema_ref()
                    .fields
                    .iter()
                    .map(|field| field.name())
                    .collect_vec();

                #[rustfmt::skip]
                table
                    .load_preset(UTF8_BORDERS_ONLY)
                    .set_header(&header);

                for row in 0..stats.num_rows() {
                    let mut values = Vec::<&str>::new();
                    for col in 0..header.len() {
                        values.push(
                            stats
                                .column(col)
                                .as_any()
                                .downcast_ref::<StringArray>()
                                .unwrap()
                                .value(row),
                        )
                    }

                    table.add_row(values);
                }

                Some(table.to_string())
            }
        };

        Ok(msg)
    }
}
