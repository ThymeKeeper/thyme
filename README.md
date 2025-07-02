# thyme

A terminal text editor written in Rust.

## Features

- Syntax highlighting for 35+ languages
- Word wrapping with proper cursor navigation
- Auto-save (after 2 seconds of inactivity)
- Customizable, hot-swappable color themes
- Adjustable margins

### Requirements

- Rust 1.70+
- A terminal

## Keybindings

### File Operations
- `Ctrl+S` - Save
- `Ctrl+Q` - Quit

### Navigation
- Arrow keys - Move cursor
- `Home`/`End` - Beginning/end of line
- `Page Up`/`Down` - Scroll

### Editing
- `Enter` - New line
- `Tab` - Insert 4 spaces
- `Backspace`/`Delete` - Delete character

### Customization
- `F1` - Help
- `F2`/`F3` - Adjust vertical margins
- `F4`/`F5` - Adjust horizontal margins
- `F6` - Toggle word wrap
- `Ctrl+L` - Change syntax highlighting language
- `Ctrl+T` - Change color theme

## Themes

Themes go in `~/.config/thyme/themes/`. Example theme:

```toml
name = "My Theme"

[colors]
background = "#1e1e1e"
foreground = "#d4d4d4"
keyword = "#569cd6"
string = "#ce9178"
comment = "#6a9955"
# See config.rs for all color options
```

## Notes

- No plugin system
- No LSP support
- No mouse support
- Just a text editor
