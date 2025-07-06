# Thyme

A terminal text editor that won't make you cry. Written in Rust.

## Features

- **Syntax highlighting** for 35+ languages with state machine-based parsing
- **Customizable themes** with live preview (includes Monokai, Dracula, Solarized Dark, Nord)
- **Mouse support** including click, drag selection, and even scroll wheel!
- **Line numbers** with absolute/relative modes
- **Undo/Redo** with intelligent action grouping
- **Configurable margins** for distraction-free writing
- **Word wrapping** with intelligent cursor movement and proper indentation preservation
- **UTF-8 support** with proper character handling
- **Clipboard integration** (copy/cut/paste)
- **Paragraph navigation** for quick document traversal
- **Bullet journal** shortcuts for task management

## Installation

### From Source

```bash
git clone https://github.com/yourusername/thyme.git
cd thyme
cargo build --release
./target/release/thyme
```

### Prerequisites

- Rust 1.70 or higher
- A modern terminal

## Usage

```bash
# Open a new file
thyme

# Open an existing file
thyme myfile.rs
```

## Key Bindings

### File Operations
- `Ctrl+S` - Save file
- `Ctrl+Q` - Quit

### Navigation
- Arrow keys - Move cursor
- `Home/End` - Beginning/end of line
- `PageUp/PageDown` - Move by page
- `Ctrl+PageUp/PageDown` - Jump between paragraphs

### Editing
- `Ctrl+A` - Select all
- `Ctrl+C/X/V` - Copy/Cut/Paste
- `Ctrl+Z/Y` - Undo/Redo
- `Tab/Shift+Tab` - Indent/Dedent

### UI Customization
- `F1` - Help
- `F2/F3` - Adjust vertical margins
- `F4/F5` - Adjust horizontal margins
- `F6` - Toggle word wrap
- `F7` - Cycle line numbers (None → Absolute → Relative)
- `Ctrl+L` - Language selection
- `Ctrl+T` - Theme selection

### Bullet Journal
- `Ctrl+←` - Insert todo bullet (□)
- `Ctrl+↓` - Insert in-progress bullet (◪)
- `Ctrl+→` - Insert done bullet (■)

## Configuration

Configuration is stored in `~/.config/thyme/config.toml`. The editor creates a default configuration on first run.

### Example Configuration

```toml
word_wrap = false
auto_save_delay_seconds = 0  # 0 = disabled
scrolloff = 3
theme_name = "Monokai"

[margins]
vertical = 0
horizontal = 0

[keybindings]
help = { code = "F1", modifiers = [] }
toggle_word_wrap = { code = "F6", modifiers = [] }
language_selection = { code = "l", modifiers = ["ctrl"] }
```

### Themes

Themes are stored in `~/.config/thyme/themes/`. The editor includes several built-in themes, but more can be added:
- Default Dark
- Monokai
- Dracula
- Solarized Dark
- Nord

## Supported Languages

Syntax highlighting is available for:

**Systems**: Rust, C, C++, Go  
**Scripting**: Python, JavaScript, TypeScript, Ruby, Perl, Lua, Bash  
**Web**: HTML, CSS, Markdown  
**Data**: JSON, TOML, YAML, XML, SQL  
**Functional**: Haskell, Clojure, Elixir, Erlang, Elm  
**Others**: Java, PHP, Swift, Kotlin, Scala, R, Dart, Dockerfile, Makefile

Language detection is automatic based on file extension.

## Technical Details

- Built with [Ratatui](https://github.com/ratatui-org/ratatui) for terminal UI
- Uses [Ropey](https://github.com/cessen/ropey) for efficient text rope data structure
- Implements gap buffer-style editing with piece table characteristics
- State machine-based syntax highlighting with incremental updates
- Zero dependencies for core editing operations

## Contributing

Pull requests welcome. Please ensure:
- Code follows Rust idioms and passes `cargo clippy`
- New features include appropriate tests
- Syntax highlighters use the existing state machine pattern

## Why "Thyme"?

Because everyone needs more thyme for editing.