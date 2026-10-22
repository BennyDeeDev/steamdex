use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) steam: SteamConfig,
    #[serde(default)]
    pub(crate) secrets_file: Option<PathBuf>,
    #[serde(default)]
    pub(crate) parsers: Vec<ParserConfig>,
    #[serde(default)]
    pub(crate) apps: Vec<AppConfig>,
}

impl Config {
    pub(crate) fn secrets_path(&self) -> PathBuf {
        self.secrets_file
            .clone()
            .unwrap_or_else(|| default_config_directory().join("secrets.json"))
    }
}

pub(crate) fn default_config_directory() -> PathBuf {
    if let Some(config_dir) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config_dir).join("steamdex");
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/steamdex")
}

#[derive(Deserialize)]
pub(crate) struct Secrets {
    pub(crate) steamgriddb_api_key: String,
}

pub(crate) fn load_secrets(path: &Path) -> Result<Secrets, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read secrets file '{}': {error}", path.display()))?;
    let secrets: Secrets = serde_json::from_str(&contents)
        .map_err(|error| format!("Could not parse secrets file '{}': {error}", path.display()))?;
    if secrets.steamgriddb_api_key.trim().is_empty() {
        return Err(format!(
            "steamgriddb_api_key is empty in secrets file '{}'.",
            path.display()
        )
        .into());
    }
    Ok(secrets)
}

#[derive(Deserialize)]
pub(crate) struct SteamConfig {
    pub(crate) directory: PathBuf,
    pub(crate) user_data_id: String,
    #[serde(default)]
    pub(crate) artwork_directory: Option<PathBuf>,
    #[serde(default)]
    pub(crate) shortcuts_file: Option<PathBuf>,
    #[serde(default)]
    pub(crate) collections_file: Option<PathBuf>,
}

impl SteamConfig {
    pub(crate) fn shortcuts_path(&self) -> PathBuf {
        self.shortcuts_file.clone().unwrap_or_else(|| {
            self.directory
                .join("userdata")
                .join(&self.user_data_id)
                .join("config")
                .join("shortcuts.vdf")
        })
    }

    pub(crate) fn grid_directory(&self) -> PathBuf {
        self.artwork_directory.clone().unwrap_or_else(|| {
            self.directory
                .join("userdata")
                .join(&self.user_data_id)
                .join("config")
                .join("grid")
        })
    }

    pub(crate) fn collections_path(&self) -> PathBuf {
        self.collections_file.clone().unwrap_or_else(|| {
            self.directory
                .join("userdata")
                .join(&self.user_data_id)
                .join("config")
                .join("cloudstorage")
                .join("cloud-storage-namespace-1.json")
        })
    }
}

#[derive(Deserialize)]
pub(crate) struct ParserConfig {
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) title_from: TitleSource,
    pub(crate) directories: Vec<PathBuf>,
    pub(crate) globs: Vec<String>,
    pub(crate) steam_shortcut: SteamShortcutConfig,
}

#[derive(Deserialize)]
pub(crate) struct AppConfig {
    pub(crate) title: String,
    pub(crate) steam_shortcut: SteamShortcutConfig,
}

#[derive(Deserialize, Clone)]
pub(crate) struct SteamShortcutConfig {
    pub(crate) executable: PathBuf,
    pub(crate) start_directory: Option<PathBuf>,
    #[serde(default)]
    pub(crate) launch_options: String,
    #[serde(default)]
    pub(crate) categories: Vec<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TitleSource {
    RootChildDirectory,
}

impl TitleSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RootChildDirectory => "root_child_directory",
        }
    }
}

pub(crate) enum ConfigLoadError {
    Read(io::Error),
    Parse(serde_json::Error),
}

pub(crate) fn load(path: &Path) -> Result<Config, ConfigLoadError> {
    let contents = fs::read_to_string(path).map_err(ConfigLoadError::Read)?;
    serde_json::from_str(&contents).map_err(ConfigLoadError::Parse)
}
