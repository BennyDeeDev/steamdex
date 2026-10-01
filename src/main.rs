use std::path::PathBuf;

use clap::{Parser, Subcommand};
use iced::{Element, widget::text};
use serde::Deserialize;

#[derive(Parser)]
#[command(
    name = "lududex",
    about = "A cross-platform steam library manager",
    long_about = "Discover games and ROMs with configurable parsers, then organize them in your Steam library.",
    version
)]

struct Cli {
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    List,
    Parse {
        #[arg(required_unless_present = "all")]
        name: Option<String>,

        #[arg(long, conflicts_with = "name")]
        all: bool,
        //TODO: add preview option
    },
}

#[derive(Deserialize)]
struct Config {
    parsers: Vec<ParserConfig>,
}

#[derive(Deserialize)]
struct ParserConfig {
    name: String,
    directory: PathBuf,
    extension: Vec<String>,
}

fn main() -> iced::Result {
    let cli = Cli::parse();

    // TODO: use .config here instead of lududex.json
    let config_path = cli.config.unwrap_or_else(|| "lududex.json".into());
    let config_file = std::fs::read_to_string(&config_path).unwrap_or_else(|error| {
        eprintln!(
            "Could not read config file '{}': {error}\nCheck that the path exists and the file is readable.",
            config_path.display()
        );
        std::process::exit(1);
    });

    let config_json: Config = serde_json::from_str(&config_file).unwrap_or_else(|error| {
        eprintln!(
            "Could not parse config file '{}': {error}",
            config_path.display()
        );
        std::process::exit(1);
    });

    match cli.command {
        Some(Command::List) => {
            if config_json.parsers.is_empty() {
                println!("No parsers configured.");
            } else {
                for parser in config_json.parsers.iter() {
                    println!("{}", parser.name);
                    println!("Directory: {}", parser.directory.display());
                    println!("Extensions: {}", parser.extension.join(", "));
                    println!();
                }
            }

            std::process::exit(0)
        }
        Some(Command::Parse { name, all }) => {
            if all {
                println!("Parsing all");
            } else if let Some(name) = name {
                let parser = config_json
                    .parsers
                    .iter()
                    .find(|p| p.name == name)
                    .unwrap_or_else(|| {
                        eprintln!("No parser named '{name}' in the config.");
                        std::process::exit(2);
                    });

                let entries = std::fs::read_dir(&parser.directory).unwrap_or_else(|error| {
                    eprintln!(
                        "Could not read directory '{}': {error}",
                        parser.directory.display()
                    );
                    std::process::exit(1);
                });

                for entry in entries {
                    let entry = entry.unwrap_or_else(|error| {
                        eprintln!("Could not read a directory entry: {error}");
                        std::process::exit(1);
                    });

                    println!("{}", entry.path().display());
                }
            }

            Ok(())
        }
        None => iced::run(update, view),
    }
}

fn update(_: &mut (), _: ()) {}

fn view(_: &()) -> Element<'_, ()> {
    text("Hello World!").into()
}
