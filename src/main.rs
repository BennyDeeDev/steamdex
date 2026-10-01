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
    Parse {
        #[arg(required_unless_present = "all")]
        name: Option<String>,

        #[arg(long, conflicts_with = "name")]
        all: bool,
    },
}

#[derive(Deserialize)]
struct Config {
    parsers: Vec<ParserConfig>,
}

#[derive(Deserialize)]
struct ParserConfig {
    name: String,
    root: String,
    extensions: Vec<String>,
}

fn main() -> iced::Result {
    let cli = Cli::parse();

    // TODO: use .config here instead of lududex.json
    let config_path = cli.config.unwrap_or_else(|| "lududex.json".into());
    let config_file = std::fs::read_to_string(&config_path).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });

    let config_json: Config = serde_json::from_str(&config_file).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });

    for parser in &config_json.parsers {
        println!("Name: {}", parser.name);
        println!("Root: {}", parser.root);
        println!("Extensions: {}", parser.extensions.join(", "));
    }

    match cli.command {
        Some(Command::Parse { name, all }) => {
            if all {
                println!("Parsing all");
            } else if let Some(name) = name {
                println!("Parsing: {name}");
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
