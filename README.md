# Thyme 🌿

A modern terminal text editor that won't waste your thyme.

## Features

Thyme is a lightweight, terminal-based text editor written in Rust that focuses on simplicity without sacrificing the essentials.

### Core Features

- **Syntax Highlighting** - Support for 35+ languages with accurate tokenization
- **Customizable Themes** - Multiple built-in themes with live preview (Ctrl+T)
- **Word Wrapping** - Smart cursor movement that actually works with wrapped lines
- **Find & Replace** - With regex-free simplicity (sometimes less is more)
- **Line Numbers** - Absolute, relative, or none - your choice
- **Mouse Support** - Click, drag, scroll - because it's the 21st century
- **UTF-8 Support** - 完全なUnicode対応 🎌
- **Configurable Margins** - For those who appreciate whitespace for distraction free coding/writing
- **Undo/Redo** - With intelligent time based grouping

### Bullet Journal Support

Because sometimes you need to organize your thoughts:
- `Ctrl+Left` - Insert todo bullet (🞎)
- `Ctrl+Down` - Insert in-progress bullet (◪) 
- `Ctrl+Right` - Insert done bullet (■)

## Installation

```bash
# Clone the repository
git clone https://github.com/thymekeeper/thyme.git
cd thyme

# Build with cargo
cargo build --release

# Run directly
cargo run --release [filename]

# Or install locally
cargo install --path .
```

## Usage

```bash
# Open a file
thyme myfile.rs

# Start with empty buffer
thyme
```

## Configuration

Thyme stores its configuration in `~/.config/thyme/config.toml`. The config file is created automatically on first run with sensible defaults.

### Example Configuration

```toml
word_wrap = false
auto_save_delay_seconds = 0  # 0 = disabled
scrolloff = 3

[margins]
vertical = 0
horizontal = 0

[gutter]
# Options: "None", "Absolute", "Relative"
mode = "None"
```

## Key Bindings

### Essential Commands
- `Ctrl+S`           - Save file
- `Ctrl+Q`           - Quit
- `Ctrl+Z/Y`         - Undo/Redo
- `F1`               - Help (comprehensive keybinding list)

### Navigation
- `Arrow Keys`       - Move caret
- `PgUp/PgDown`      - Scroll by page
- `Ctrl+PgUp/PgDown` - Jump between paragraphs
- `Home/End`         - Beginning/end of line

### Editing
- `Ctrl+A`           - Select all
- `Ctrl+C/X/V`       - Copy/Cut/Paste
- `Tab/Shift+Tab`    - Indent/Dedent
- `Ctrl+F`           - Find & Replace

### Finding/replaceing
- `Ctrl+F`           - Find
- `Ctrl+Alt+F`       - Find previous
- `Ctrl+H`           - Replace
- `Ctrl+Alt+H`       - Replace all

### Customization
- `F2/F3`            - Adjust vertical margins
- `F4/F5`            - Adjust horizontal margins
- `F6`               - Toggle word wrap
- `F7`               - Cycle line numbers (None → Absolute → Relative)
- `Ctrl+L`           - Change syntax highlighting language
- `Ctrl+T`           - Change color theme

## Supported Languages

Thyme provides syntax highlighting for:

**Popular Languages**: Rust, Python, JavaScript, TypeScript, Go, Java, C, C++, Swift, Kotlin

**Web Technologies**: HTML, CSS, JSON, XML, YAML, TOML

**Scripting**: Bash, Ruby, Perl, Lua, R

**Functional**: Haskell, Clojure, Elixir, Erlang, Elm, Scala

**Config Files**: Dockerfile, Makefile, SQL

**And More**: Markdown, Vim Script, PHP, Dart

## Themes

Thyme includes several built-in themes:
- Default Dark (for the minimalists)
- Monokai
- Dracula  
- Solarized Dark
- Nord

Themes are stored in `~/.config/thyme/themes/` as TOML files. Feel free to create your own!

## Building from Source

Requirements:
- Rust 1.70+ 
- A modern terminal

---

*Note: No actual herbs were harmed in the making of this editor.*