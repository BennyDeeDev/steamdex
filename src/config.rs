use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) parsers: Vec<ParserConfig>,
}

#[derive(Deserialize)]
pub(crate) struct ParserConfig {
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) title_from: TitleSource,
    pub(crate) directories: Vec<PathBuf>,
    pub(crate) globs: Vec<String>,
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
