# Steamdex

**Steamdex** is a cross-platform game library manager written in Rust, designed
as a declarative alternative to Steam ROM Manager.

It discovers games and ROMs using configurable parsers and integrates them into
your Steam library as non-Steam shortcuts, complete with metadata and artwork.

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

The configuration has a `steam_target` and a list of `parsers`. Each parser
includes its file-matching rules and a `steam_shortcut` definition:

```json
{
  "steam_target": {
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
        "steam_categories": ["ROMs"]
      }
    }
  ]
}
```

`steam_target.directory` points to the Steam installation, and
`steam_target.user_data_id` identifies the userdata folder. For each parser,
`steam_shortcut` contains the executable, optional start directory, launch
options, and Steam categories. `${file_path}` can be used in launch options to
refer to a matched file.

Each parser's `globs` field is a list of [globset patterns](https://docs.rs/globset/latest/globset/#syntax):

Steamdex recursively walks each configured directory and matches each file's
**filename only** against these patterns. For example, `*.rom` matches ROM
filenames at any depth; folder names do not affect matching. Patterns are
case-sensitive. Invalid glob patterns are reported as configuration errors.
If a configured directory cannot be read or a walk encounters an error,
Steamdex reports it and continues with other roots, but exits unsuccessfully
after returning any files it found.

The `enabled` field controls whether a parser runs when you use `steamdex parse`
without a name. You can still run a disabled parser explicitly with
`steamdex parse "Parser Name"`.

## AI Disclosure

Steamdex's hard parts are handwritten everything else is reviewed by me.
I use AI to write trivial logic, research and understand code from other
projects, and assist with supporting tasks like drafting error messages,
editing Markdown, and generating tests.

Ludusavi is one of Steamdex's inspirations, and AI has helped me explore and
understand its code. Feedback from experienced Rustaceans is very welcome and
greatly appreciated.
