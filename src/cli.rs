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

        let output = if preview {
            parse_and_preview_shortcuts(parser, &sgdb).await
        } else {
            parse_and_write_shortcuts(parser, steam_config, &sgdb).await
        }
        .unwrap_or_else(|error| {
            eprintln!("Error [{}]: {error}", parser.name);
            std::process::exit(1);
        });
        for error in &output.errors {
            eprintln!("Error [{}]: {error}", parser.name);
        }
        if !output.errors.is_empty() {
            std::process::exit(1);
        }
        return;
    }

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
        let result = if preview {
            parse_and_preview_shortcuts(parser, &sgdb).await
        } else {
            parse_and_write_shortcuts(parser, steam_config, &sgdb).await
        };
        match result {
            Ok(output) => {
                for error in &output.errors {
                    eprintln!("Error [{}]: {error}", parser.name);
                }
                if !output.errors.is_empty() {
                    had_errors = true;
                }
            }
            Err(error) => {
                eprintln!("Error [{}]: {error}", parser.name);
                had_errors = true;
            }
        }
    }
    if had_errors {
        std::process::exit(1);
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

        let output = parse_and_delete_shortcuts(parser, steam_config).unwrap_or_else(|error| {
            eprintln!("Error [{}]: {error}", parser.name);
            std::process::exit(1);
        });
        for error in &output.errors {
            eprintln!("Error [{}]: {error}", parser.name);
        }
        if !output.errors.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    if parsers.is_empty() {
        println!("No parsers configured.");
        return;
    }

    let mut had_errors = false;
    for parser in parsers {
        match parse_and_delete_shortcuts(parser, steam_config) {
            Ok(output) => {
                for error in &output.errors {
                    eprintln!("Error [{}]: {error}", parser.name);
                }
                if !output.errors.is_empty() {
                    had_errors = true;
                }
            }
            Err(error) => {
                eprintln!("Error [{}]: {error}", parser.name);
                had_errors = true;
            }
        }
    }
    if had_errors {
        std::process::exit(1);
    }
}

async fn parse_and_preview_shortcuts(
    parser_config: &ParserConfig,
    sgdb: &SteamGridDbClient,
) -> Result<parser::ParseOutput, Box<dyn std::error::Error>> {
    println!("Preview: {}", parser_config.name);

    let mut output = parser::parse(parser_config)?;

    if output.titles.is_empty() {
        println!("  No matching titles found.");
        return Ok(output);
    }

    for title in &output.titles {
        print_title(title);

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

    Ok(output)
}

fn parse_and_delete_shortcuts(
    parser_config: &ParserConfig,
    steam_config: &SteamConfig,
) -> Result<parser::ParseOutput, Box<dyn std::error::Error>> {
    println!("Delete: {}", parser_config.name);

    let mut output = parser::parse(parser_config)?;

    if output.titles.is_empty() {
        println!("  No matching titles found.");
        return Ok(output);
    }

    let path = steam_config.shortcuts_path();
    match steam::delete_shortcuts(&path, &parser_config.steam_shortcut, &output.titles) {
        Ok(()) => println!("Shortcut deletion completed at {}", path.display()),
        Err(error) => output.errors.push(error),
    }

    Ok(output)
}

async fn parse_and_write_shortcuts(
    parser_config: &ParserConfig,
    steam_config: &SteamConfig,
    sgdb: &SteamGridDbClient,
) -> Result<parser::ParseOutput, Box<dyn std::error::Error>> {
    println!("Parser: {}", parser_config.name);

    let mut output = parser::parse(parser_config)?;

    if output.titles.is_empty() {
        println!("  No matching titles found.");
        return Ok(output);
    }

    let Some(executable) = parser_config.steam_shortcut.executable.to_str() else {
        output
            .errors
            .push("Steam shortcut executable path is not valid UTF-8".into());
        return Ok(output);
    };

    for title in &mut output.titles {
        print_title(title);

        let game = match sgdb.search_first_game(&title.title).await {
            Ok(Some(game)) => Some(game),
            Ok(None) => {
                println!("  SteamGridDB: no match");
                None
            }
            Err(error) => {
                println!("  SteamGridDB: lookup failed");
                output.errors.push(
                    format!("SteamGridDB search failed for '{}': {error}", title.title).into(),
                );
                None
            }
        };

        if let Some(game) = game {
            let (_, shortcut_id) = steam::shortcut_app_ids(executable, &title.title);

            println!("  SteamGridDB: {} (ID: {})", game.name, game.id);
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
                                println!("  {artwork_type}: download failed");
                                output.errors.push(
                                    format!(
                                        "Could not download {artwork_type} for '{}': {error}",
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
                                println!("  {artwork_type}: wrote {}", path.display());
                                if matches!(artwork_type, ArtworkType::Icon) {
                                    title.icon_path = Some(path);
                                }
                            }
                            Err(error) => {
                                println!("  {artwork_type}: write failed");
                                output.errors.push(
                                    format!(
                                        "Could not write {artwork_type} for '{}': {error}",
                                        title.title
                                    )
                                    .into(),
                                );
                            }
                        }
                    }
                    Ok(None) => println!("  {artwork_type}: no artwork found"),
                    Err(error) => {
                        println!("  {artwork_type}: lookup failed");
                        output.errors.push(
                            format!(
                                "Could not look up {artwork_type} for '{}': {error}",
                                title.title
                            )
                            .into(),
                        );
                    }
                }
            }
        }
    }

    println!();
    let path = steam_config.shortcuts_path();
    match steam::write_shortcuts(&path, &parser_config.steam_shortcut, &output.titles) {
        Ok(()) => println!("Wrote shortcuts to {}", path.display()),
        Err(error) => output.errors.push(error),
    }

    Ok(output)
}

fn print_title(title: &parser::ParsedTitles) {
    println!("\n{}:", title.title);
    for file in &title.files {
        println!("  File: {}", file.display());
    }
}
