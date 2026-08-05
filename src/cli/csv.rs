use arrow_csv::ReaderBuilder;
use fallible_iterator::{FallibleIterator, IteratorExt};
use itertools::Itertools;
use reedline_repl_rs::clap::{ArgAction, ArgMatches, FromArgMatches, Subcommand};
use std::collections::VecDeque;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use storage::RecordBatch;
use storage::storage::entry::Entry;
use storage::storage::file::File;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a table from a CSV file
    Import {
        /// Table to create from the CSV file
        #[arg(required = true)]
        table: String,

        /// Path to the CSV file
        #[arg(required = true)]
        path: PathBuf,

        /// Required to overwrite an existing table
        #[arg(long, action(ArgAction::SetTrue))]
        overwrite: bool,
    },
    /// Create a CSV file from a table
    Export {
        /// Table to create the CSV file from
        #[arg(required = true)]
        table: String,

        /// Path to the CSV file to create
        path: Option<PathBuf>,

        /// Required to overwrite an existing file
        #[arg(action(ArgAction::SetTrue))]
        force: bool,
    },
}

pub fn commands(args: ArgMatches, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
    Ok(Command::from_arg_matches(&args)?.run(file)?)
}

impl Command {
    pub(crate) fn run(self, file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
        match self {
            Command::Import {
                path,
                table,
                overwrite,
            } => import(file, path, table, overwrite),
            _ => unreachable!(),
        }
    }

    // fn export(self, _file: &mut File) -> Result<Option<String>, Box<dyn Error>> {
    //     //
    //     Ok(None)
    // }
}

fn import(
    file: &mut File,
    path: PathBuf,
    table: String,
    overwrite: bool,
) -> Result<Option<String>, Box<dyn Error>> {
    let csv = std::fs::File::open(&path)?;
    let schema = arrow_csv::infer_schema_from_files(
        &[path.as_path().to_string_lossy().to_string()],
        b',',
        None,
        true,
    )?;

    let reader = ReaderBuilder::new(Arc::new(schema))
        .with_header(true)
        .build(csv)?;

    let mut batches: VecDeque<_> = reader
        .map_ok(RecordBatch::from)
        .transpose_into_fallible()
        .collect()?;

    let mut n = 0;
    if let Some(mut batch) = batches.pop_front() {
        batch.extend(&batches);

        // println!("{} bytes", batch.get_array_memory_size());

        n = batch.num_rows();
        let txn = file.txn()?;

        match txn.entry(&table)? {
            Entry::Occupied(entry) => {
                if overwrite == false {
                    Err(redb::TableError::TableExists(table))?
                }

                entry.remove_entry()?.insert_entry(batch)?;
            }
            Entry::Vacant(entry) => {
                entry.insert_entry(batch)?;
            }
        };

        txn.commit()?;
    }

    Ok(Some(format!("{} rows added", n)))
}
