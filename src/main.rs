use std::{fs, io, path::PathBuf};

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
    title_from: TitleSource,
    directories: Vec<PathBuf>,
    globs: Vec<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum TitleSource {
    RootChildDirectory,
}

impl TitleSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::RootChildDirectory => "root_child_directory",
        }
    }
}

struct ParsedTitles {
    title: String,
    files: Vec<PathBuf>,
}

struct ParseOutput {
    titles: Vec<ParsedTitles>,
    errors: Vec<Box<dyn std::error::Error>>,
}

fn parse(parser: &ParserConfig) -> Result<ParseOutput, Box<dyn std::error::Error>> {
    if parser.directories.is_empty() {
        return Err("parser has no directories configured".into());
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in &parser.globs {
        builder.add(Glob::new(pattern)?);
    }
    let globs = builder.build()?;

    let mut output = ParseOutput {
        titles: Vec::new(),
        errors: Vec::new(),
    };

    for directory in &parser.directories {
        match fs::metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                output.errors.push(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("'{}' is not a directory", directory.display()),
                    )
                    .into(),
                );
                continue;
            }
            Err(error) => {
                output.errors.push(
                    io::Error::new(
                        error.kind(),
                        format!("could not inspect '{}': {error}", directory.display()),
                    )
                    .into(),
                );
                continue;
            }
        }
        for title_entry in walkdir::WalkDir::new(directory)
            .min_depth(1)
            .max_depth(1)
            .follow_links(true)
        {
            let title_entry = match title_entry {
                Ok(entry) => entry,
                Err(error) => {
                    output.errors.push(error.into());
                    continue;
                }
            };

            if title_entry.file_type().is_dir() {
                let mut parsed_title = ParsedTitles {
                    title: title_entry.file_name().to_string_lossy().into_owned(),
                    files: Vec::new(),
                };

                for file_entry in walkdir::WalkDir::new(title_entry.path())
                    .max_depth(100)
                    .follow_links(true)
                {
                    let file_entry = match file_entry {
                        Ok(file_entry) => file_entry,
                        Err(error) => {
                            output.errors.push(error.into());
                            continue;
                        }
                    };

                    if file_entry.file_type().is_file()
                        && file_entry
                            .path()
                            .file_name()
                            .is_some_and(|name| globs.is_match(std::path::Path::new(name)))
                    {
                        parsed_title.files.push(file_entry.into_path());
                    }
                }

                if !parsed_title.files.is_empty() {
                    output.titles.push(parsed_title);
                }
            }
        }
    }

    Ok(output)
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
                for (index, parser) in config_json.parsers.iter().enumerate() {
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
        }
        Command::Parse { name } => {
            //TODO: add rayon to multithread multiple parsers
            if let Some(name) = name {
                let parser = config_json
                    .parsers
                    .iter()
                    .find(|p| p.name == name)
                    .unwrap_or_else(|| {
                        eprintln!("No parser named '{name}' in the config.");
                        std::process::exit(2);
                    });

                let output = parse(parser).unwrap_or_else(|error| {
                    eprintln!("Could not parse '{}': {error}", parser.name);
                    std::process::exit(1);
                });
                for title in output.titles {
                    println!("{}:", title.title);
                    for file in title.files {
                        println!("{}", file.display());
                    }
                }
                if !output.errors.is_empty() {
                    for error in output.errors {
                        eprintln!("Could not parse '{}': {error}", parser.name);
                    }
                    std::process::exit(1);
                }
            } else {
                let mut enabled_parsers = config_json
                    .parsers
                    .iter()
                    .filter(|parser| parser.enabled)
                    .peekable();

                if enabled_parsers.peek().is_none() {
                    println!("No enabled parsers configured.");
                }

                let mut had_errors = false;
                for parser in enabled_parsers {
                    println!("{}:", parser.name);
                    let output = parse(parser).unwrap_or_else(|error| {
                        eprintln!("Could not parse '{}': {error}", parser.name);
                        std::process::exit(1);
                    });
                    for title in output.titles {
                        println!("{}:", title.title);
                        for file in title.files {
                            println!("{}", file.display());
                        }
                    }
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
    }
}
