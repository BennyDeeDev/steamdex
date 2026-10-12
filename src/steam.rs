use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;

use crate::{
    config::{SteamConfig, SteamShortcutConfig},
    parser::ParsedTitles,
    steamgriddb::ArtworkType,
    vdf::{self, Value},
};

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

fn get_string_field<'a>(fields: &'a IndexMap<String, Value>, wanted: &str) -> Option<&'a str> {
    for (key, value) in fields {
        if key.eq_ignore_ascii_case(wanted) {
            return match value {
                Value::String(value) => Some(value.as_str()),
                _ => None,
            };
        }
    }
    None
}

fn build_shortcut(
    config: &SteamShortcutConfig,
    game: &ParsedTitles,
) -> Result<IndexMap<String, Value>, Box<dyn Error>> {
    let exe = config
        .executable
        .to_str()
        .ok_or("Steam shortcut executable path is not valid UTF-8")?;
    let start_directory = config
        .start_directory
        .as_deref()
        .or_else(|| config.executable.parent())
        .unwrap_or_else(|| Path::new(""))
        .to_str()
        .ok_or("Steam shortcut start directory is not valid UTF-8")?;
    let file = game
        .files
        .first()
        .ok_or("Parsed title has no matching files")?;
    let file_path = file.to_str().ok_or("Game file path is not valid UTF-8")?;
    let launch_options = config.launch_options.replace("${file_path}", file_path);
    let icon = match game.icon_path.as_deref() {
        Some(path) => path
            .to_str()
            .ok_or("Steam shortcut icon path is not valid UTF-8")?
            .to_owned(),
        None => String::new(),
    };
    let tags = config
        .categories
        .iter()
        .enumerate()
        .map(|(index, category)| (index.to_string(), Value::String(category.clone())))
        .collect();
    let (_, appid) = shortcut_app_ids(exe, &game.title);

    Ok(IndexMap::from([
        ("appid".to_owned(), Value::Int(appid)),
        ("appname".to_owned(), Value::String(game.title.clone())),
        ("exe".to_owned(), Value::String(exe.to_owned())),
        (
            "StartDir".to_owned(),
            Value::String(start_directory.to_owned()),
        ),
        ("LaunchOptions".to_owned(), Value::String(launch_options)),
        ("icon".to_owned(), Value::String(icon)),
        ("tags".to_owned(), Value::Object(tags)),
    ]))
}

pub(crate) fn write_shortcuts(
    path: &Path,
    config: &SteamShortcutConfig,
    titles: &[ParsedTitles],
) -> Result<(), Box<dyn Error>> {
    let mut shortcuts = vdf::read_vdf_shortcuts(path)?;
    let mut slots_by_shortcut = HashMap::new();
    for (slot, shortcut) in &shortcuts {
        let Value::Object(fields) = shortcut else {
            continue;
        };
        let (Some(exe), Some(appname)) = (
            get_string_field(fields, "exe"),
            get_string_field(fields, "appname"),
        ) else {
            continue;
        };

        slots_by_shortcut
            .entry((exe.to_owned(), appname.to_owned()))
            .or_insert_with(|| slot.clone());
    }
    let mut highest_slot = shortcuts
        .keys()
        .filter_map(|slot| slot.parse::<u64>().ok())
        .max();

    for title in titles {
        let fields = build_shortcut(config, title)?;

        let exe = get_string_field(&fields, "exe").ok_or("New shortcut is missing exe")?;
        let appname =
            get_string_field(&fields, "appname").ok_or("New shortcut is missing appname")?;

        let key = (exe.to_owned(), appname.to_owned());
        let slot = match slots_by_shortcut.get(&key) {
            Some(slot) => slot.clone(),
            None => {
                let next_slot = match highest_slot {
                    Some(slot) => slot.checked_add(1).ok_or("Shortcut slot overflow")?,
                    None => 0,
                };
                highest_slot = Some(next_slot);
                let slot = next_slot.to_string();
                slots_by_shortcut.insert(key, slot.clone());
                slot
            }
        };

        shortcuts.insert(slot, Value::Object(fields));
    }

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    vdf::write_vdf_shortcuts(path, &shortcuts)
}

pub(crate) fn delete_shortcuts(
    path: &Path,
    config: &SteamShortcutConfig,
    titles: &[ParsedTitles],
) -> Result<(), Box<dyn Error>> {
    let mut shortcuts = vdf::read_vdf_shortcuts(path)?;

    let exe = config
        .executable
        .to_str()
        .ok_or("Steam shortcut executable path is not valid UTF-8")?;

    let title_names: HashSet<&str> = titles.iter().map(|title| title.title.as_str()).collect();
    let original_len = shortcuts.len();
    shortcuts.retain(|_, shortcut| {
        let Value::Object(fields) = shortcut else {
            return true;
        };

        if get_string_field(fields, "exe") != Some(exe) {
            return true;
        }

        let matching_title =
            get_string_field(fields, "appname").is_some_and(|name| title_names.contains(name));

        !matching_title
    });

    if shortcuts.len() == original_len {
        return Ok(());
    }

    vdf::write_vdf_shortcuts(path, &shortcuts)
}
