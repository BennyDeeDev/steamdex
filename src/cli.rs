use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::{self, ConfigLoadError, ParserConfig},
    parser,
    steamgriddb::{ArtworkType, SteamGridDbClient},
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

pub(crate) async fn run() {
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
        Command::List => list_command(&config_json),
        Command::Parse { name } => parse_command(&config_json.parsers, name).await,
    }
}

fn list_command(config: &crate::config::Config) {
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

async fn parse_command(parsers: &[ParserConfig], name: Option<String>) {
    // TODO: add rayon to multithread multiple parsers

    if let Some(name) = name {
        let parser = parsers
            .iter()
            .find(|parser| parser.name == name)
            .unwrap_or_else(|| {
                eprintln!("No parser named '{name}' in the config.");
                std::process::exit(2);
            });

        let sgdb = SteamGridDbClient::new().unwrap_or_else(|error| {
            eprintln!("Could not initialize SteamGridDB: {error}");
            std::process::exit(1);
        });

        let output = parse_and_lookup_artwork(parser, &sgdb)
            .await
            .unwrap_or_else(|error| {
                eprintln!("Could not parse '{}': {error}", parser.name);
                std::process::exit(1);
            });
        for error in &output.errors {
            eprintln!("Could not parse '{}': {error}", parser.name);
        }
        if !output.errors.is_empty() {
            std::process::exit(1);
        }
    } else {
        let mut enabled_parsers = parsers.iter().filter(|parser| parser.enabled).peekable();

        if enabled_parsers.peek().is_none() {
            println!("No enabled parsers configured.");
            return;
        }

        let sgdb = SteamGridDbClient::new().unwrap_or_else(|error| {
            eprintln!("Could not initialize SteamGridDB: {error}");
            std::process::exit(1);
        });

        let mut had_errors = false;
        for parser in enabled_parsers {
            println!("{}:", parser.name);
            match parse_and_lookup_artwork(parser, &sgdb).await {
                Ok(output) => {
                    for error in &output.errors {
                        eprintln!("Could not parse '{}': {error}", parser.name);
                    }
                    if !output.errors.is_empty() {
                        had_errors = true;
                    }
                }
                Err(error) => {
                    eprintln!("Could not parse '{}': {error}", parser.name);
                    had_errors = true;
                }
            }
        }
        if had_errors {
            std::process::exit(1);
        }
    }
}

async fn parse_and_lookup_artwork(
    parser_config: &ParserConfig,
    sgdb: &SteamGridDbClient,
) -> Result<parser::ParseOutput, Box<dyn std::error::Error>> {
    let mut output = parser::parse(parser_config)?;

    print_output(&output.titles);

    for title in &output.titles {
        let game = match sgdb.search_first_game(&title.title).await {
            Ok(Some(game)) => game,
            Ok(None) => {
                println!("No SteamGridDB match for '{}'.", title.title);
                continue;
            }
            Err(error) => {
                output.errors.push(
                    format!("SteamGridDB search failed for '{}': {error}", title.title).into(),
                );
                continue;
            }
        };

        println!("SteamGridDB match: {} ({})", game.name, game.id);
        for artwork_type in [
            ArtworkType::Tall,
            ArtworkType::Long,
            ArtworkType::Hero,
            ArtworkType::Logo,
            ArtworkType::Icon,
        ] {
            match sgdb.first_artwork(game.id, artwork_type).await {
                Ok(Some(asset)) => {
                    println!("  {artwork_type:?}: artwork {} — {}", asset.id, asset.url)
                }
                Ok(None) => println!("  {artwork_type:?}: no result"),
                Err(error) => {
                    output.errors.push(
                        format!(
                            "Could not get {artwork_type:?} for '{}': {error}",
                            title.title
                        )
                        .into(),
                    );
                }
            }
        }
    }

    Ok(output)
}

fn print_output(titles: &[parser::ParsedTitles]) {
    for title in titles {
        println!("{}:", title.title);
        for file in &title.files {
            println!("{}", file.display());
        }
    }
}
