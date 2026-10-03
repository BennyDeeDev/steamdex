use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::{self, ConfigLoadError, ParserConfig},
    parser,
};

#[derive(Parser)]
#[command(
    name = "steamdex",
    about = "A cross-platform steam library manager",
    long_about = "Discover games and ROMs with configurable parsers, then organize them in your Steam library.",
    version
)]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    List,
    Parse {
        name: Option<String>,
        // TODO: add preview option
    },
}

pub(crate) fn run() {
    let cli = Cli::parse();

    // TODO: use .config here instead of steamdex.json
    let config_path = cli.config.unwrap_or_else(|| "steamdex.json".into());
    let config_json = config::load(&config_path).unwrap_or_else(|error| {
        match error {
            ConfigLoadError::Read(error) => eprintln!(
                "Could not read config file '{}': {error}\nCheck that the path exists and the file is readable.",
                config_path.display()
            ),
            ConfigLoadError::Parse(error) => eprintln!(
                "Could not parse config file '{}': {error}",
                config_path.display()
            ),
        }
        std::process::exit(1);
    });

    match cli.command {
        Command::List => list(&config_json),
        Command::Parse { name } => parse_command(&config_json.parsers, name),
    }
}

fn list(config: &crate::config::Config) {
    if config.parsers.is_empty() {
        println!("No parsers configured.");
        return;
    }

    for (index, parser) in config.parsers.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("{}", parser.name);
        println!("  Enabled: {}", parser.enabled);
        println!("  Title from: {}", parser.title_from.as_str());
        println!("  Directories:");
        if parser.directories.is_empty() {
            println!("    (none)");
        } else {
            for directory in &parser.directories {
                println!("    - {}", directory.display());
            }
        }
        println!("  Globs: {}", parser.globs.join(", "));
    }
}

fn parse_command(parsers: &[ParserConfig], name: Option<String>) {
    // TODO: add rayon to multithread multiple parsers
    if let Some(name) = name {
        let parser = parsers
            .iter()
            .find(|parser| parser.name == name)
            .unwrap_or_else(|| {
                eprintln!("No parser named '{name}' in the config.");
                std::process::exit(2);
            });

        let output = parser::parse(parser).unwrap_or_else(|error| {
            eprintln!("Could not parse '{}': {error}", parser.name);
            std::process::exit(1);
        });
        print_output(output.titles);
        if !output.errors.is_empty() {
            for error in output.errors {
                eprintln!("Could not parse '{}': {error}", parser.name);
            }
            std::process::exit(1);
        }
    } else {
        let mut enabled_parsers = parsers.iter().filter(|parser| parser.enabled).peekable();

        if enabled_parsers.peek().is_none() {
            println!("No enabled parsers configured.");
        }

        let mut had_errors = false;
        for parser in enabled_parsers {
            println!("{}:", parser.name);
            let output = parser::parse(parser).unwrap_or_else(|error| {
                eprintln!("Could not parse '{}': {error}", parser.name);
                std::process::exit(1);
            });
            print_output(output.titles);
            for error in output.errors {
                eprintln!("Could not parse '{}': {error}", parser.name);
                had_errors = true;
            }
        }
        if had_errors {
            std::process::exit(1);
        }
    }
}

fn print_output(titles: Vec<parser::ParsedTitles>) {
    for title in titles {
        println!("{}:", title.title);
        for file in title.files {
            println!("{}", file.display());
        }
    }
}
