# Steamdex

Steamdex is a declarative Steam library manager for ROMs and non-Steam apps, written in Rust.

## Installation

### Nix flake (recommended)

Run directly:

```sh
nix run github:BennyDeeDev/steamdex -- --help
```

Or install it:

```sh
nix profile install github:BennyDeeDev/steamdex
```

To build the flake locally, run `nix build` from a checkout. The executable will be at `./result/bin/steamdex`.

### Build from source

Requires Rust and Cargo.

```sh
git clone https://github.com/BennyDeeDev/steamdex.git
cd steamdex
cargo build --release
./target/release/steamdex --help
```

## Configuration

Create `~/.config/steamdex/config.json` (or `$XDG_CONFIG_HOME/steamdex/config.json`):

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
  ],
  "parsers": [
    {
      "name": "ROMs",
      "enabled": true,
      "title_from": "root_child_directory",
      "directories": ["/path/to/roms"],
      "globs": ["*.rom", "*.bin"],
      "steam_shortcut": {
        "executable": "/path/to/emulator",
        "launch_options": "\"${file_path}\"",
        "categories": ["ROMs"]
      }
    }
  ]
}
```

`user_data_id` is the folder name under Steam's `userdata/` directory. Parsers discover games from subdirectories and expand `${file_path}` to the first matching file. `globs` uses [globset syntax](https://docs.rs/globset/latest/globset/#syntax).

### SteamGridDB API key

Steamdex uses [SteamGridDB](https://www.steamgriddb.com/) to find games and artwork. To get a key, sign in, open [Profile → Preferences → API](https://www.steamgriddb.com/profile/preferences/api), and click **Generate API Key**.

Copy it into `~/.config/steamdex/secrets.json`:

```json
{
  "steamgriddb_api_key": "your-api-key"
}
```

## Usage

```sh
steamdex list                  # Show configuration
steamdex parse --preview       # Preview changes
steamdex parse                 # Add or update shortcuts
steamdex parse "Steam Link"    # Process one app or parser
steamdex delete "Steam Link"   # Remove one shortcut
steamdex delete                # Remove configured shortcuts
```

Close Steam before modifying shortcuts.

## Roadmap

- [ ] Build a functional CLI with configuration support.
- [ ] Add local cache.
- [ ] Add profiles for preconfigured setups, selectors, and game executable lookup.
- [ ] Support glob-regex and full SRM Compatibility.
- [ ] Add an Iced UI.
- [ ] Support SteamCDN as provider.

## AI Disclosure

Steamdex's hard parts are handwritten everything else is reviewed by me. I use AI to write trivial logic, research and understand code from other projects, and assist with supporting tasks like drafting error messages, editing Markdown, and generating tests.

Ludusavi is one of Steamdex's inspirations, and AI has helped me explore and understand its code. Feedback from experienced Rustaceans is very welcome and greatly appreciated.
