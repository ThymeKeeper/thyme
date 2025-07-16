// src/lib.rs
pub mod buffer;
pub mod config;
pub mod cursor;
pub mod syntax;
pub mod text_utils;
pub mod unicode_utils;

// Re-export the main types for convenience
pub use buffer::Buffer;
pub use config::{Config, Theme};
pub use cursor::{Cursor, CursorMovement};
pub use syntax::{SyntaxHighlighter, TokenType, SyntaxToken};
pub use text_utils::{wrap_line, detect_language_from_path, get_language_display_name, get_supported_languages};
pub use unicode_utils::{char_display_width, str_display_width};
