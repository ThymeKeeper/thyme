# Thyme

A terminal text editor that respects both your time and your retinas.

## What is Thyme?

Thyme is a modern terminal text editor built in Rust. It's the editor for people who want syntax highlighting that actually works, word wrapping that doesn't make you want to wrap your keyboard around something, and the ability to see your code without squinting.

## Features

- **35+ language syntax highlighting** - Because code should be colorful
- **Intelligent word wrapping** - Cursor movement that makes sense, even when your lines don't
- **Mouse support** - Yes, in the terminal. Welcome to the future
- **Live theme switching** - Preview themes in real-time instead of editing config files like it's 1999
- **Configurable margins** - For when you want your code/document centered, like a work of art
- **Multiple gutter modes** - Absolute, relative, or none. We don't judge
- **Clean status bar** - Shows what you need, skips what you don't

## Why Thyme?

We're not saying other terminal editors are bad. Nano is everywhere, which is great when you're SSH'd into a server from 2003. Micro has plugins for days, perfect for when you want your text editor to also make coffee. 

Thyme just edits text. But it does it with:
- Rust's memory safety (no random segfaults)
- Modern terminal features (your terminal supports true color, why shouldn't your editor?)
- Reasonable defaults (Ctrl+S saves, imagine that)
- A codebase you can actually read

## Installation

```bash
git clone https://github.com/thymekeeper/thyme
cd thyme
cargo build --release
sudo cp target/release/thyme /usr/local/bin/
```

## Usage

```bash
thyme filename.rs
```

### Key Bindings

The basics work like you'd expect:
- `Ctrl+S` - Save
- `Ctrl+Q` - Quit
- `Ctrl+A` - Select all
- `Ctrl+C/X/V` - Copy/Cut/Paste
- `Ctrl+Z/Y` - Undo/Redo

The fun stuff:
- `F1` - Help (yes, it actually helps)
- `F2/F3` - Adjust vertical margins
- `F4/F5` - Adjust horizontal margins  
- `F6` - Toggle word wrap
- `F7` - Cycle gutter modes
- `Ctrl+L` - Change language syntax
- `Ctrl+T` - Browse themes

### Mouse Support

Click to move cursor. Drag to select. Scroll to... scroll. Revolutionary, we know.

## Configuration

Config lives at `~/.config/thyme/config.toml`. It's TOML because it's readable by humans.

```toml
[margins]
horizontal = 2
vertical = 1

word_wrap = true
auto_save_delay_seconds = 0  # 0 = disabled

# Customize keybindings if you're feeling adventurous
[keybindings]
help = { code = "F1", modifiers = [] }
```

## Themes

Themes live in `~/.config/thyme/themes/`. Ships with:
- Monokai (the classic)
- Dracula (for vampires)
- Solarized Dark (easy on the eyes)
- Nord (imported from Scandinavia)

## Building from Source

Requirements:
- Rust 1.70+
- A terminal emulator from this decade

```bash
cargo build --release
```

## Philosophy

Thyme follows the philosophy of "do one thing well, but make sure that thing includes syntax highlighting, word wrapping, mouse support, themes, and configurable margins." 

It's a text editor that:
- Starts fast
- Responds instantly
- Doesn't require a PhD in configuration
- Actually fits in a terminal

## What Thyme Doesn't Do

- Multiple cursors (one cursor, one destiny)
- Plugin system (it's complete as-is)
- Integrated terminal (you're already in one)
- Code completion (that's what language servers are for)
- Web browsing (looking at you, Emacs)

## Contributing

Found a bug? Have a feature request that won't turn Thyme into a different editor? PRs welcome!

Please keep in mind:
- Thyme aims to be a focused text editor
- Performance matters
- Code clarity matters more
- Tests are your friends

## License

MIT - Because sharing is caring

---

*Thyme: For when you just want to edit text, but you want to enjoy it.*