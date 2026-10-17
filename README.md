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

The configuration has a `steam` section and a list of `parsers`. Each parser
includes its file-matching rules and a `steam_shortcut` definition:

```json
{
  "steam": {
    "directory": "/path/to/Steam",
    "user_data_id": "123456789",
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

## AI Disclosure

Steamdex's hard parts are handwritten everything else is reviewed by me.
I use AI to write trivial logic, research and understand code from other
projects, and assist with supporting tasks like drafting error messages,
editing Markdown, and generating tests.

Ludusavi is one of Steamdex's inspirations, and AI has helped me explore and
understand its code. Feedback from experienced Rustaceans is very welcome and
greatly appreciated.
