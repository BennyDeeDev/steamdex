use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use indexmap::IndexMap;
use serde_json::{Value as JsonValue, json};

use crate::{
    config::{SteamConfig, SteamShortcutConfig},
    steamgriddb::ArtworkType,
    vdf::{self, Value},
};

pub(crate) struct Shortcut {
    pub(crate) title: String,
    pub(crate) config: SteamShortcutConfig,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) icon_path: Option<PathBuf>,
}

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
    fs::create_dir_all(&grid_directory).map_err(|error| {
        format!(
            "Could not create artwork directory '{}': {error}",
            grid_directory.display()
        )
    })?;

    let path = grid_directory.join(format!("{artwork_id}{suffix}.{extension}"));
    fs::write(&path, bytes)
        .map_err(|error| format!("Could not write artwork '{}': {error}", path.display()))?;
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

fn build_shortcut(shortcut: &Shortcut) -> Result<IndexMap<String, Value>, Box<dyn Error>> {
    let config = &shortcut.config;
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
    let icon = match shortcut.icon_path.as_deref() {
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
    let (_, appid) = shortcut_app_ids(exe, &shortcut.title);

    Ok(IndexMap::from([
        ("appid".to_owned(), Value::Int(appid)),
        ("appname".to_owned(), Value::String(shortcut.title.clone())),
        ("exe".to_owned(), Value::String(exe.to_owned())),
        (
            "StartDir".to_owned(),
            Value::String(start_directory.to_owned()),
        ),
        (
            "LaunchOptions".to_owned(),
            Value::String(config.launch_options.clone()),
        ),
        ("icon".to_owned(), Value::String(icon)),
        ("tags".to_owned(), Value::Object(tags)),
    ]))
}

pub(crate) fn write_shortcuts(path: &Path, entries: &[Shortcut]) -> Result<(), Box<dyn Error>> {
    let mut shortcuts = vdf::read_vdf_shortcuts(path).map_err(|error| {
        format!(
            "Could not load shortcuts file '{}': {error}",
            path.display()
        )
    })?;
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

    for shortcut in entries {
        let fields = build_shortcut(shortcut).map_err(|error| {
            format!("Could not build shortcut for '{}': {error}", shortcut.title)
        })?;

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
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Could not create shortcuts directory '{}': {error}",
                parent.display()
            )
        })?;
    }

    vdf::write_vdf_shortcuts(path, &shortcuts)
}

pub(crate) fn write_categories(
    path: &Path,
    categories: &[String],
    app_ids: &[u32],
) -> Result<(), Box<dyn Error>> {
    if categories.is_empty() || app_ids.is_empty() {
        return Ok(());
    }

    let mut entries: Vec<(String, JsonValue)> = serde_json::from_slice(&fs::read(path)?)?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let mut changed = false;

    for category in categories {
        let wanted = category.to_lowercase();
        let mut found = None;

        for (index, (key, entry)) in entries.iter().enumerate() {
            if !key.starts_with("user-collections.") {
                continue;
            }

            let Some(value) = entry["value"].as_str() else {
                continue;
            };

            let collection: JsonValue = serde_json::from_str(value)?;

            if collection["name"]
                .as_str()
                .is_some_and(|name| name.to_lowercase() == wanted)
            {
                found = Some((index, collection));
                break;
            }
        }

        let (index, mut collection) = match found {
            Some(found) => found,
            None => {
                let id = format!("steamdex-{}-{}", now.as_nanos(), entries.len());
                let key = format!("user-collections.{id}");

                entries.push((
                    key.clone(),
                    json!({
                        "key": key,
                    }),
                ));

                (
                    entries.len() - 1,
                    json!({
                        "id": id,
                        "name": category,
                        "added": [],
                    }),
                )
            }
        };

        let original = collection.clone();

        if collection["added"].is_null() {
            collection["added"] = json!([]);
        }

        let added = collection["added"]
            .as_array_mut()
            .ok_or("Collection 'added' must be an array")?;

        for &app_id in app_ids {
            let value = json!(app_id);

            if !added.contains(&value) {
                added.push(value);
            }
        }

        if collection != original {
            entries[index].1["value"] = JsonValue::String(serde_json::to_string(&collection)?);
            entries[index].1["timestamp"] = json!(now.as_secs());
            changed = true;
        }
    }

    if changed {
        fs::write(path, serde_json::to_vec(&entries)?)?;
    }

    Ok(())
}

pub(crate) fn delete_shortcuts(path: &Path, entries: &[Shortcut]) -> Result<(), Box<dyn Error>> {
    let mut shortcuts = vdf::read_vdf_shortcuts(path).map_err(|error| {
        format!(
            "Could not load shortcuts file '{}': {error}",
            path.display()
        )
    })?;

    let mut targets = HashSet::new();
    for shortcut in entries {
        let exe = shortcut
            .config
            .executable
            .to_str()
            .ok_or("Steam shortcut executable path is not valid UTF-8")?;
        targets.insert((exe, shortcut.title.as_str()));
    }

    let original_len = shortcuts.len();
    shortcuts.retain(|_, shortcut| {
        let Value::Object(fields) = shortcut else {
            return true;
        };

        let (Some(exe), Some(appname)) = (
            get_string_field(fields, "exe"),
            get_string_field(fields, "appname"),
        ) else {
            return true;
        };

        !targets.contains(&(exe, appname))
    });

    if shortcuts.len() == original_len {
        return Ok(());
    }

    vdf::write_vdf_shortcuts(path, &shortcuts)
}
