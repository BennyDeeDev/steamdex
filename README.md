# Steamdex

**Steamdex** is a cross-platform steam library manager written in Rust, designed
as a declarative alternative to Steam ROM Manager.

Rather than relying on GUI-defined parsing rules, Steamdex puts declarative
configuration first, making game discovery and library management reproducible,
portable, and easy to automate.

## Roadmap

- [ ] Build a functional CLI with configuration support.
- [ ] Add local cache.
- [ ] Add profiles for preconfigured setups, selectors, and game executable lookup.
- [ ] Support glob-regex and full SRM Compatibility.
- [ ] Add an Iced UI.
- [ ] Support SteamCDN as provider.

## Parser file matching

The configuration has a `steam` section and optional `parsers` and `apps` lists.
Parsers discover titles from files; apps declare shortcuts directly. Each parser
includes its file-matching rules and a `steam_shortcut` definition:

```json
{
  "steam": {
    "directory": "/path/to/Steam",
    "user_data_id": "123456789"
  },
  "parsers": [
    {
      "name": "Sample ROM Library",
      "enabled": true,
      "title_from": "root_child_directory",
      "directories": ["test-data/sample-library-rom"],
      "globs": ["*.rom", "*.bin"],
      "steam_shortcut": {
        "executable": "/path/to/emulator",
        "start_directory": null,
        "launch_options": "\"${file_path}\"",
        "categories": ["ROMs"]
      }
    }
  ]
}
```

Each parser's `globs` field is a list of [globset patterns](https://docs.rs/globset/latest/globset/#syntax).

## Explicit apps

Declare apps directly in the top-level `apps` list in the same configuration file:

```json
{
  "steam": {
    "directory": "/path/to/Steam",
    "user_data_id": "123456789"
  },
  "apps": [
    {
      "title": "Steam Link",
      "steam_shortcut": {
        "executable": "/usr/bin/flatpak",
        "launch_options": "run com.valvesoftware.SteamLink",
        "categories": ["Media"]
      }
    }
  ]
}
```

Shortcut settings may omit `start_directory` (defaults to the executable's parent),
`launch_options` (defaults to an empty string), and `categories` (defaults to an
empty list).

`list` displays configured apps and their launch settings.

- `parse` processes enabled parsers and all explicit apps.
- `parse --preview` previews those shortcuts without writing files.
- `parse "Steam Link"` processes an app by its title.
- `delete "Steam Link"` deletes that app's shortcut.
- `delete` processes all configured parsers and apps, including disabled parsers.

A named command can also select a parser. If the name matches multiple entries,
the command reports an ambiguity.

File parsers expand `${file_path}` in launch options using the first matched file.
Explicit apps use their configured launch options literally.

SteamGridDB features require a `secrets.json` file containing your API key:

```json
{
  "steamgriddb_api_key": "your-api-key"
}
```

Preview, add or update, and remove the Steam Link shortcut:

```bash
steamdex parse "Steam Link" --preview
steamdex parse "Steam Link"
steamdex delete "Steam Link"
```

## AI Disclosure

Steamdex's hard parts are handwritten everything else is reviewed by me.
I use AI to write trivial logic, research and understand code from other
projects, and assist with supporting tasks like drafting error messages,
editing Markdown, and generating tests.

Ludusavi is one of Steamdex's inspirations, and AI has helped me explore and
understand its code. Feedback from experienced Rustaceans is very welcome and
greatly appreciated.
