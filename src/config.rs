use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) steam: SteamConfig,
    pub(crate) parsers: Vec<ParserConfig>,
}

#[derive(Deserialize)]
pub(crate) struct SteamConfig {
    pub(crate) directory: PathBuf,
    pub(crate) user_data_id: String,
    #[serde(default)]
    pub(crate) artwork_directory: Option<PathBuf>,
}

impl SteamConfig {
    pub(crate) fn grid_directory(&self) -> PathBuf {
        self.artwork_directory.clone().unwrap_or_else(|| {
            self.directory
                .join("userdata")
                .join(&self.user_data_id)
                .join("config")
                .join("grid")
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
pub(crate) struct SteamShortcutConfig {
    pub(crate) executable: PathBuf,
    pub(crate) start_directory: Option<PathBuf>,
    pub(crate) launch_options: String,
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
