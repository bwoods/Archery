use clap_verbosity_flag::Verbosity;
use reedline_repl_rs::clap::{Parser, Subcommand};
use reedline_repl_rs::{CallBackMap, Repl};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use storage::File;

pub mod csv;
pub mod db;
pub mod help;
pub mod obj;

#[derive(Parser, Debug)]
#[command(about, version, disable_help_subcommand = true)]
pub struct Args {
    /// Path to a storage file
    ///
    /// The file will be created if it does not already
    /// exist. Defaults to a temporary file.
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Noun,

    #[command(flatten)]
    verbosity: Verbosity,
}

#[derive(Debug, Subcommand)]
pub enum Noun {
    /// File information and maintenance
    #[command(subcommand)]
    File(db::Command),
    /// Import/export of tables
    #[command(subcommand)]
    Csv(csv::Command),
    /// Import/export
    #[command(subcommand)]
    Obj(obj::Command),
    /// Enter a command shell
    #[clap(alias = "sh")]
    Script,
    /// Interactive exploration of commands
    Help(help::UI),
}

fn cli(mut file: File, command: Noun) -> Result<Option<String>, Box<dyn Error>> {
    let outro = match command {
        Noun::Csv(verb) => verb.run(&mut file)?,
        Noun::Obj(verb) => verb.run(&mut file)?,
        Noun::File(verb) => verb.run(&mut file)?,
        Noun::Script => {
            let mut callbacks: CallBackMap<File, Box<dyn Error>> = HashMap::new();
            callbacks.insert("file".to_string(), db::verbs);
            callbacks.insert("csv".to_string(), csv::verbs);
            callbacks.insert("obj".to_string(), obj::verbs);

            let mut repl = Repl::new(file)
                .with_derived::<Args>(callbacks)
                .with_history(".history".into(), 500)
                .with_partial_completions(true)
                .with_quick_completions(true);

            repl.run().map_err(|err| Box::new(err))?;
            None
        }
        Noun::Help(ui) => help::run(file, ui, cli)?,
    };

    Ok(outro)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    SimpleLogger::new()
        .with_level(args.verbosity.into())
        .init()?;

    let file = match args.file {
        None => File::temporary()?,
        Some(file) => File::path(file)?,
    };

    let outro = cli(file, args.command)?;
    Ok(println!("{}", outro.unwrap_or_default())) // TODO: log-levels
}
