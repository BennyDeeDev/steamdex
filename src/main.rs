use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use globset::{Glob, GlobSetBuilder};
use serde::Deserialize;

#[derive(Parser)]
#[command(
    name = "steamdex",
    about = "A cross-platform steam library manager",
    long_about = "Discover games and ROMs with configurable parsers, then organize them in your Steam library.",
    version
)]

struct Cli {
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    List,
    Parse {
        name: Option<String>,
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
    enabled: bool,
    directory: PathBuf,
    globs: Vec<String>,
}

fn parse(parser: &ParserConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &parser.globs {
        builder.add(Glob::new(pattern)?);
    }
    let globs = builder.build()?;

    let metadata = fs::metadata(&parser.directory)?;
    if !metadata.is_dir() {
        return Err("path is not a folder".into());
    }

    let files: Vec<PathBuf> = walkdir::WalkDir::new(&parser.directory)
        .max_depth(100)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                let error_path = error.path().unwrap_or(parser.directory.as_path());
                eprintln!("Could not walk '{}': {error}", error_path.display());
                None
            }
        })
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .file_name()
                    .is_some_and(|name| globs.is_match(std::path::Path::new(name)))
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    for file in files {
        println!("{}", file.display());
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    // TODO: use .config here instead of steamdex.json
    let config_path = cli.config.unwrap_or_else(|| "steamdex.json".into());
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
        Command::List => {
            if config_json.parsers.is_empty() {
                println!("No parsers configured.");
            } else {
                for parser in config_json.parsers.iter() {
                    println!("{}", parser.name);
                    println!();
                    println!("Enabled: {}", parser.enabled);
                    println!("Directory: {}", parser.directory.display());
                    println!("Globs: {}", parser.globs.join(", "));
                    println!();
                    println!("=======================");
                }
            }
        }
        Command::Parse { name } => {
            if let Some(name) = name {
                let parser = config_json
                    .parsers
                    .iter()
                    .find(|p| p.name == name)
                    .unwrap_or_else(|| {
                        eprintln!("No parser named '{name}' in the config.");
                        std::process::exit(2);
                    });

                parse(parser).unwrap_or_else(|error| {
                    eprintln!(
                        "Could not parse '{}' at '{}': {error}",
                        parser.name,
                        parser.directory.display()
                    );
                    std::process::exit(1);
                });
            } else {
                let mut enabled_parsers = config_json
                    .parsers
                    .iter()
                    .filter(|parser| parser.enabled)
                    .peekable();

                if enabled_parsers.peek().is_none() {
                    println!("No enabled parsers configured.");
                }

                for parser in enabled_parsers {
                    println!("{}:", parser.name);
                    parse(parser).unwrap_or_else(|error| {
                        eprintln!(
                            "Could not parse '{}' at '{}': {error}",
                            parser.name,
                            parser.directory.display()
                        );
                        std::process::exit(1);
                    });
                }
            }
        }
    }
}
