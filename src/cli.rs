use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::{self, ConfigLoadError, ParserConfig, SteamConfig},
    parser, steam,
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
        /// Show discovered titles, files, and SteamGridDB matches without writing shortcuts or artwork.
        #[arg(long)]
        preview: bool,
    },
    Delete {
        name: Option<String>, // TODO: add preview option
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
        Command::Parse { name, preview } => {
            parse_command(&config_json.parsers, &config_json.steam, name, preview).await
        }
        Command::Delete { name } => delete_command(&config_json.parsers, &config_json.steam, name),
    }
}

fn list_command(config: &crate::config::Config) {
    println!("Steam:");
    println!("  Directory: {}", config.steam.directory.display());
    println!("  User data ID: {}", config.steam.user_data_id);
    println!(
        "  Artwork directory: {}",
        config.steam.grid_directory().display()
    );
    println!(
        "  Shortcuts file: {}",
        config.steam.shortcuts_path().display()
    );

    if config.parsers.is_empty() {
        println!();
        println!("No parsers configured.");
        return;
    }

    for parser in &config.parsers {
        println!();
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
        println!("  Steam shortcut:");
        println!(
            "    Executable: {}",
            parser.steam_shortcut.executable.display()
        );
        println!(
            "    Start directory: {}",
            parser
                .steam_shortcut
                .start_directory
                .as_deref()
                .map_or_else(|| "(default)".to_owned(), |path| path.display().to_string())
        );
        println!(
            "    Launch options: {}",
            parser.steam_shortcut.launch_options
        );
        println!(
            "    Categories: {}",
            parser.steam_shortcut.categories.join(", ")
        );
    }
}

async fn parse_command(
    parsers: &[ParserConfig],
    steam_config: &SteamConfig,
    name: Option<String>,
    preview: bool,
) {
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

        if preview {
            parse_and_preview_shortcuts(parser, steam_config, &sgdb)
                .await
                .unwrap_or_else(|error| {
                    eprintln!("Could not preview '{}': {error}", parser.name);
                    std::process::exit(1);
                });
            return;
        }

        let output = parse_and_write_shortcuts(parser, steam_config, &sgdb)
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

        if preview {
            let mut had_errors = false;
            for parser in enabled_parsers {
                if let Err(error) = parse_and_preview_shortcuts(parser, steam_config, &sgdb).await {
                    eprintln!("Could not preview '{}': {error}", parser.name);
                    had_errors = true;
                }
            }
            if had_errors {
                std::process::exit(1);
            }
            return;
        }

        let mut had_errors = false;
        for parser in enabled_parsers {
            println!("{}:", parser.name);
            match parse_and_write_shortcuts(parser, steam_config, &sgdb).await {
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

fn delete_command(parsers: &[ParserConfig], steam_config: &SteamConfig, name: Option<String>) {
    if let Some(name) = name {
        let parser = parsers
            .iter()
            .find(|parser| parser.name == name)
            .unwrap_or_else(|| {
                eprintln!("No parser named '{name}' in the config.");
                std::process::exit(2);
            });

        parse_and_delete_shortcuts(parser, steam_config).unwrap_or_else(|error| {
            eprintln!("Could not delete shortcuts for '{}': {error}", parser.name);
            std::process::exit(1);
        });
    } else {
        if parsers.is_empty() {
            println!("No parsers configured.");
            return;
        }

        let mut had_errors = false;
        for parser in parsers {
            println!("{}:", parser.name);
            if let Err(error) = parse_and_delete_shortcuts(parser, steam_config) {
                eprintln!("Could not delete shortcuts for '{}': {error}", parser.name);
                had_errors = true;
            }
        }
        if had_errors {
            std::process::exit(1);
        }
    }
}

async fn parse_and_preview_shortcuts(
    parser_config: &ParserConfig,
    steam_config: &SteamConfig,
    sgdb: &SteamGridDbClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Preview: {}", parser_config.name);

    let mut output = parser::parse(parser_config)?;

    if output.titles.is_empty() {
        println!("  No matching titles found.");
    }

    for title in &output.titles {
        println!("\n{}:", title.title);
        for file in &title.files {
            println!("  File: {}", file.display());
        }

        match sgdb.search_first_game(&title.title).await {
            Ok(Some(game)) => {
                println!("  SteamGridDB: {} (ID: {})", game.name, game.id);
            }
            Ok(None) => {
                println!("  SteamGridDB: no match");
            }
            Err(error) => {
                println!("  SteamGridDB: lookup failed");
                output.errors.push(
                    format!("SteamGridDB search failed for '{}': {error}", title.title).into(),
                );
            }
        }
    }

    println!();
    for error in &output.errors {
        eprintln!("Could not preview '{}': {error}", parser_config.name);
    }

    let file_count: usize = output.titles.iter().map(|title| title.files.len()).sum();

    println!(
        "Found {} titles, {} matching files, {} errors.",
        output.titles.len(),
        file_count,
        output.errors.len(),
    );
    println!(
        "Target shortcuts file: {}",
        steam_config.shortcuts_path().display(),
    );

    if !output.errors.is_empty() {
        return Err(format!(
            "Preview for '{}' encountered {} error(s)",
            parser_config.name,
            output.errors.len(),
        )
        .into());
    }

    Ok(())
}

fn parse_and_delete_shortcuts(
    parser_config: &ParserConfig,
    steam_config: &SteamConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = parser::parse(parser_config)?;
    for error in &output.errors {
        eprintln!("Could not parse '{}': {error}", parser_config.name);
    }

    if !output.titles.is_empty() {
        let path = steam_config.shortcuts_path();
        steam::delete_shortcuts(&path, &parser_config.steam_shortcut, &output.titles)?;
        println!("Processed shortcut deletion at {}", path.display());
    }

    if !output.errors.is_empty() {
        return Err(format!("Parser '{}' encountered errors", parser_config.name).into());
    }

    Ok(())
}

async fn parse_and_write_shortcuts(
    parser_config: &ParserConfig,
    steam_config: &SteamConfig,
    sgdb: &SteamGridDbClient,
) -> Result<parser::ParseOutput, Box<dyn std::error::Error>> {
    let mut output = parser::parse(parser_config)?;
    let executable = parser_config
        .steam_shortcut
        .executable
        .to_str()
        .ok_or("Steam shortcut executable path is not valid UTF-8")?;

    print_output(&output.titles);

    for title in &mut output.titles {
        let game = match sgdb.search_first_game(&title.title).await {
            Ok(Some(game)) => Some(game),
            Ok(None) => {
                println!("No SteamGridDB match for '{}'.", title.title);
                None
            }
            Err(error) => {
                output.errors.push(
                    format!("SteamGridDB search failed for '{}': {error}", title.title).into(),
                );
                None
            }
        };

        if let Some(game) = game {
            let (_, shortcut_id) = steam::shortcut_app_ids(executable, &title.title);

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
                        let bytes = match sgdb.download_image(&asset.url).await {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                output.errors.push(
                                    format!(
                                        "Could not download {artwork_type:?} for '{}': {error}",
                                        title.title
                                    )
                                    .into(),
                                );
                                continue;
                            }
                        };

                        match steam::write_artwork(
                            steam_config,
                            shortcut_id,
                            artwork_type,
                            asset.mime.as_deref(),
                            &bytes,
                        ) {
                            Ok(path) => {
                                println!("  {artwork_type:?}: wrote {}", path.display());
                                if matches!(artwork_type, ArtworkType::Icon) {
                                    title.icon_path = Some(path);
                                }
                            }
                            Err(error) => output.errors.push(
                                format!(
                                    "Could not write {artwork_type:?} for '{}': {error}",
                                    title.title
                                )
                                .into(),
                            ),
                        }
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
    }

    if !output.titles.is_empty() {
        let path = steam_config.shortcuts_path();
        steam::write_shortcuts(&path, &parser_config.steam_shortcut, &output.titles)?;
        println!("Wrote shortcuts to {}", path.display());
    }

    Ok(output)
}

fn print_output(titles: &[parser::ParsedTitles]) {
    for title in titles {
        println!("{}:", title.title);
        for file in &title.files {
            println!("  {}", file.display());
        }
    }
}
