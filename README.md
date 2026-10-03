# Steamdex

**Steamdex** is a cross-platform game library manager written in Rust, designed
as a declarative alternative to Steam ROM Manager.

It discovers games and ROMs using configurable parsers and integrates them into
your Steam library as non-Steam shortcuts, complete with metadata and artwork.

Rather than relying on GUI-defined parsing rules, Steamdex puts declarative
configuration first, making game discovery and library management reproducible,
portable, and easy to automate.

## Roadmap

1. Build a functional CLI with configuration support.
2. Add profiles for preconfigured setups, selectors, and game executable lookup.
3. Add an Iced UI.

## Parser file matching

Each parser's `globs` field is a list of [globset patterns](https://docs.rs/globset/latest/globset/#syntax):

```json
{
  "name": "Sample ROM Library",
  "directory": "test-data/sample-library-rom",
  "globs": ["*.rom", "*.bin"]
}
```

Steamdex recursively walks the configured directory and matches each file's
**filename only** against these patterns. For example, `*.rom` matches ROM
filenames at any depth; folder names do not affect matching. Patterns are
case-sensitive. Invalid glob patterns are reported as configuration errors.

## AI Disclosure

Steamdex's logic is handwritten and reviewed by me. I'm still learning Rust, so
I use AI to help research and understand code from other projects, along with
small supporting tasks like drafting error messages, editing Markdown, and
generating tests. Ludusavi is one of Steamdex's inspirations, and AI has helped
me explore and understand its code. Feedback from experienced Rustaceans is
very welcome and greatly appreciated.
