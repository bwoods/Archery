use reedline_repl_rs::clap::{Parser, Subcommand};
use reedline_repl_rs::{CallBackMap, Repl};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use storage::storage::File;

pub mod csv;
pub mod db;
pub mod obj;

#[derive(Parser, Debug)]
#[command(
    name = "storage",
    version,
    about = "Command line manipulation of storage",
    next_line_help = true
)]
pub struct CLI {
    #[arg(required = true)]
    pub file: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Enter a command shell
    Shell,

    /// File information and maintenance
    #[command(subcommand)]
    File(db::Command),

    /// Import/export of tables
    #[command(subcommand)]
    Csv(csv::Command),

    /// Import/export
    #[command(subcommand)]
    Obj(obj::Command),
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = CLI::parse();
    let mut file = File::path(cli.file)?;

    let outro = match cli.command {
        Command::Csv(command) => command.run(&mut file)?,
        Command::Obj(command) => command.run(&mut file)?,
        Command::File(command) => command.run(&mut file)?,
        Command::Shell => {
            let mut callbacks: CallBackMap<File, Box<dyn Error>> = HashMap::new();
            callbacks.insert("file".to_string(), db::commands);
            callbacks.insert("csv".to_string(), csv::commands);
            callbacks.insert("obj".to_string(), obj::commands);

            let mut repl = Repl::new(file)
                .with_derived::<CLI>(callbacks)
                // .with_quick_completions(true)
                .with_history(".history".into(), 500);
            return repl.run().map_err(|err| err.into());
        }
    };

    Ok(println!("{}", outro.unwrap_or_default())) // TODO: log-levels
}
