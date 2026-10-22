use std::{collections::HashMap, error::Error, path::PathBuf};

use clap::{Parser, Subcommand};

use crate::{
    config::{self, Config, ConfigLoadError, SteamShortcutConfig},
    parser,
    steam::{self, Shortcut},
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
        /// Parser name or app title. Omit to process enabled parsers and all apps.
        name: Option<String>,
        /// Preview shortcuts and SteamGridDB matches without writing files.
        #[arg(long)]
        preview: bool,
    },
    Delete {
        /// Parser name or app title. Omit to process all configured entries.
        name: Option<String>, // TODO: add preview option
    },
}

pub(crate) async fn run() {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .unwrap_or_else(|| config::default_config_directory().join("config.json"));
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
        Command::Parse { name, preview } => parse_command(&config_json, name, preview).await,
        Command::Delete { name } => delete_command(&config_json, name),
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
    println!(
        "  Collections file: {}",
        config.steam.collections_path().display()
    );

    if config.parsers.is_empty() {
        println!();
        println!("No parsers configured.");
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
        print_shortcut_config(&parser.steam_shortcut);
    }

    println!();
    println!("Apps:");
    if config.apps.is_empty() {
        println!("  (none)");
    }
    for app in &config.apps {
        println!();
        println!("{}", app.title);
        print_shortcut_config(&app.steam_shortcut);
    }
}

fn print_shortcut_config(shortcut: &SteamShortcutConfig) {
    println!("  Steam shortcut:");
    println!("    Executable: {}", shortcut.executable.display());
    println!(
        "    Start directory: {}",
        shortcut
            .start_directory
            .as_deref()
            .map_or_else(|| "(default)".to_owned(), |path| path.display().to_string())
    );
    println!("    Launch options: {}", shortcut.launch_options);
    println!("    Categories: {}", shortcut.categories.join(", "));
}

struct ShortcutOutput {
    shortcuts: Vec<Shortcut>,
    errors: Vec<Box<dyn Error>>,
}

fn collect_shortcuts(
    config: &Config,
    name: Option<&str>,
    include_disabled: bool,
) -> Result<ShortcutOutput, Box<dyn Error>> {
    let parsers: Vec<_> = config
        .parsers
        .iter()
        .filter(|parser| match name {
            Some(name) => parser.name == name,
            None => include_disabled || parser.enabled,
        })
        .collect();
    let apps: Vec<_> = config
        .apps
        .iter()
        .filter(|app| match name {
            Some(name) => app.title == name,
            None => true,
        })
        .collect();

    if let Some(name) = name {
        match parsers.len() + apps.len() {
            0 => return Err(format!("No parser or app named '{name}' in the config.").into()),
            1 => {}
            _ => {
                return Err(format!(
                    "More than one parser or app is named '{name}'. Use unique parser names and app titles."
                )
                .into());
            }
        }
    }

    let mut output = ShortcutOutput {
        shortcuts: Vec::new(),
        errors: Vec::new(),
    };

    for parser_config in parsers {
        println!("Parser: {}", parser_config.name);
        let parsed = match parser::parse(parser_config) {
            Ok(parsed) => parsed,
            Err(error) => {
                output
                    .errors
                    .push(format!("[{}]: {error}", parser_config.name).into());
                continue;
            }
        };
        for error in parsed.errors {
            output
                .errors
                .push(format!("[{}]: {error}", parser_config.name).into());
        }

        for title in parsed.titles {
            let Some(file_path) = title.files.first().and_then(|file| file.to_str()) else {
                output.errors.push(
                    format!(
                        "[{}]: '{}' has no matching UTF-8 file path",
                        parser_config.name, title.title
                    )
                    .into(),
                );
                continue;
            };
            let mut shortcut_config = parser_config.steam_shortcut.clone();
            shortcut_config.launch_options = shortcut_config
                .launch_options
                .replace("${file_path}", file_path);
            output.shortcuts.push(Shortcut {
                title: title.title,
                config: shortcut_config,
                files: title.files,
                icon_path: title.icon_path,
            });
        }
    }

    for app in apps {
        output.shortcuts.push(Shortcut {
            title: app.title.clone(),
            config: app.steam_shortcut.clone(),
            files: Vec::new(),
            icon_path: None,
        });
    }

    Ok(output)
}

async fn parse_command(config: &Config, name: Option<String>, preview: bool) {
    let mut output = collect_shortcuts(config, name.as_deref(), false).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    if output.shortcuts.is_empty() {
        println!("No shortcuts found.");
        report_errors(&output.errors);
        return;
    }

    let secrets = config::load_secrets(&config.secrets_path()).unwrap_or_else(|error| {
        eprintln!("Could not initialize SteamGridDB: {error}");
        std::process::exit(1);
    });
    let sgdb = SteamGridDbClient::new(secrets.steamgriddb_api_key);
    let mut app_ids_by_categories = HashMap::<Vec<String>, Vec<u32>>::new();

    for shortcut in &mut output.shortcuts {
        print_shortcut(shortcut);
        let Some(executable) = shortcut.config.executable.to_str() else {
            output
                .errors
                .push(format!("'{}': executable path is not valid UTF-8", shortcut.title).into());
            continue;
        };
        let (_, shortcut_id) = steam::shortcut_app_ids(executable, &shortcut.title);
        if !preview {
            app_ids_by_categories
                .entry(shortcut.config.categories.clone())
                .or_default()
                .push(shortcut_id as u32);
        }

        let game = match sgdb.search_first_game(&shortcut.title).await {
            Ok(Some(game)) => {
                println!("  SteamGridDB: {} (ID: {})", game.name, game.id);
                game
            }
            Ok(None) => {
                println!("  SteamGridDB: no match");
                continue;
            }
            Err(error) => {
                println!("  SteamGridDB: lookup failed");
                output.errors.push(
                    format!(
                        "SteamGridDB search failed for '{}': {error}",
                        shortcut.title
                    )
                    .into(),
                );
                continue;
            }
        };
        if preview {
            continue;
        }

        for artwork_type in [
            ArtworkType::Tall,
            ArtworkType::Long,
            ArtworkType::Hero,
            ArtworkType::Logo,
            ArtworkType::Icon,
        ] {
            let asset = match sgdb.first_artwork(game.id, artwork_type).await {
                Ok(Some(asset)) => asset,
                Ok(None) => {
                    println!("  {artwork_type}: no artwork found");
                    continue;
                }
                Err(error) => {
                    println!("  {artwork_type}: lookup failed");
                    output.errors.push(
                        format!(
                            "Could not look up {artwork_type} for '{}': {error}",
                            shortcut.title
                        )
                        .into(),
                    );
                    continue;
                }
            };
            let bytes = match sgdb.download_image(&asset.url).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("  {artwork_type}: download failed");
                    output.errors.push(
                        format!(
                            "Could not download {artwork_type} for '{}': {error}",
                            shortcut.title
                        )
                        .into(),
                    );
                    continue;
                }
            };
            match steam::write_artwork(
                &config.steam,
                shortcut_id,
                artwork_type,
                asset.mime.as_deref(),
                &bytes,
            ) {
                Ok(path) => {
                    println!("  {artwork_type}: wrote {}", path.display());
                    if matches!(artwork_type, ArtworkType::Icon) {
                        shortcut.icon_path = Some(path);
                    }
                }
                Err(error) => {
                    println!("  {artwork_type}: write failed");
                    output.errors.push(
                        format!(
                            "Could not write {artwork_type} for '{}': {error}",
                            shortcut.title
                        )
                        .into(),
                    );
                }
            }
        }
    }

    if !preview {
        let path = config.steam.shortcuts_path();
        match steam::write_shortcuts(&path, &output.shortcuts) {
            Ok(()) => {
                println!("Wrote shortcuts to {}", path.display());
                let collections_path = config.steam.collections_path();
                for (categories, app_ids) in app_ids_by_categories {
                    if let Err(error) =
                        steam::write_categories(&collections_path, &categories, &app_ids)
                    {
                        output.errors.push(
                            format!(
                                "Could not write collections '{}': {error}",
                                collections_path.display()
                            )
                            .into(),
                        );
                    }
                }
            }
            Err(error) => output.errors.push(error),
        }
    }
    report_errors(&output.errors);
}

fn delete_command(config: &Config, name: Option<String>) {
    let mut output = collect_shortcuts(config, name.as_deref(), true).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2);
    });
    if output.shortcuts.is_empty() {
        println!("No shortcuts found.");
        report_errors(&output.errors);
        return;
    }
    for shortcut in &output.shortcuts {
        print_shortcut(shortcut);
    }
    let path = config.steam.shortcuts_path();
    match steam::delete_shortcuts(&path, &output.shortcuts) {
        Ok(()) => println!("Shortcut deletion completed at {}", path.display()),
        Err(error) => output.errors.push(error),
    }
    report_errors(&output.errors);
}

fn print_shortcut(shortcut: &Shortcut) {
    println!("\n{}:", shortcut.title);
    for file in &shortcut.files {
        println!("  File: {}", file.display());
    }
    print_shortcut_config(&shortcut.config);
}

fn report_errors(errors: &[Box<dyn Error>]) {
    for error in errors {
        eprintln!("Error: {error}");
    }
    if !errors.is_empty() {
        std::process::exit(1);
    }
}
