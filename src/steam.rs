use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;

use crate::{config::SteamConfig, steamgriddb::ArtworkType, vdf::Value};

pub(crate) fn shortcut_app_ids(exe: &str, app_name: &str) -> (u64, i32) {
    let crc = crc32fast::hash(format!("{exe}{app_name}").as_bytes());
    let short_id = crc | 0x8000_0000;
    let long_id = (u64::from(short_id) << 32) | 0x0200_0000;

    (long_id, short_id as i32)
}

pub(crate) fn write_artwork(
    steam: &SteamConfig,
    shortcut_id: i32,
    artwork_type: ArtworkType,
    mime: Option<&str>,
    bytes: &[u8],
) -> Result<PathBuf, Box<dyn Error>> {
    let extension = match mime {
        Some("image/jpeg") => "jpg",
        Some("image/png") => "png",
        Some("image/webp") => "png",
        Some("image/tga" | "image/x-tga") => "tga",
        Some("image/vnd.microsoft.icon" | "image/x-icon") => "ico",
        Some(other) => return Err(format!("unsupported artwork MIME type '{other}'").into()),
        None => return Err("SteamGridDB artwork is missing its MIME type".into()),
    };

    let suffix = match artwork_type {
        ArtworkType::Tall => "p",
        ArtworkType::Long => "",
        ArtworkType::Hero => "_hero",
        ArtworkType::Logo => "_logo",
        ArtworkType::Icon => "_icon",
    };

    let artwork_id = shortcut_id as u32;
    let grid_directory = steam.grid_directory();
    fs::create_dir_all(&grid_directory)?;

    let path = grid_directory.join(format!("{artwork_id}{suffix}.{extension}"));
    fs::write(&path, bytes)?;
    Ok(path)
}

#[allow(dead_code)]
pub(crate) fn write_shortcuts(
    path: &Path,
    shortcuts: &IndexMap<String, Value>,
) -> Result<(), Box<dyn Error>> {
    todo!("write shortcuts to Steam")
}
