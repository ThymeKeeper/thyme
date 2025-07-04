// src/text_utils.rs
//
// Shared text manipulation utilities

/// Wraps a line of text to fit within a specified width, preserving word boundaries when possible.
/// Returns a vector of (segment, start_position) tuples where start_position is the character
/// position in the original text (not including any added indentation).
pub fn wrap_line(text: &str, width: usize) -> Vec<(String, usize)> {
    if width == 0 {
        return vec![(text.to_string(), 0)];
    }

    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    
    if chars.is_empty() {
        return vec![(String::new(), 0)];
    }

    // Calculate the indentation of the first line
    let mut indent_len = 0;
    for &ch in &chars {
        if ch == ' ' || ch == '\t' {
            indent_len += 1;
        } else {
            break;
        }
    }
    
    // Create the indent string for continuation lines
    let indent_string: String = chars[..indent_len].iter().collect();
    let indent_width = indent_len; // For spaces this is accurate, tabs would need special handling
    
    let mut start_pos = 0;
    let mut is_first_line = true;
    
    while start_pos < chars.len() {
        // Calculate effective width for this line
        let effective_width = if is_first_line {
            width
        } else {
            // For continuation lines, reduce width by indent amount
            width.saturating_sub(indent_width)
        };
        
        if effective_width == 0 {
            // If no space left after indentation, use at least 1 character width
            let end_pos = (start_pos + 1).min(chars.len());
            let segment: String = if is_first_line {
                chars[start_pos..end_pos].iter().collect()
            } else {
                let line_content: String = chars[start_pos..end_pos].iter().collect();
                format!("{}{}", indent_string, line_content)
            };
            result.push((segment, start_pos));
            start_pos = end_pos;
            is_first_line = false;
            continue;
        }
        
        let mut end_pos = (start_pos + effective_width).min(chars.len());
        
        // If we're not at the end of the text, try to break at a word boundary
        if end_pos < chars.len() {
            // Look backwards from end_pos to find a space
            let mut break_pos = end_pos;
            for i in (start_pos..end_pos).rev() {
                if chars[i] == ' ' {
                    break_pos = i;
                    break;
                }
            }
            
            // If we found a space and it's not too close to the start, use it
            if break_pos > start_pos && (break_pos - start_pos) > effective_width / 4 {
                end_pos = break_pos;
            }
        }
        
        // Extract the segment
        let segment: String = if is_first_line {
            // First line - use as is
            chars[start_pos..end_pos].iter().collect()
        } else {
            // Continuation line - prepend the indentation
            let line_content: String = chars[start_pos..end_pos].iter().collect();
            format!("{}{}", indent_string, line_content)
        };
        
        // Always use the actual start position from the original text
        result.push((segment, start_pos));
        
        // Move to the next segment, skipping any spaces at the break point ONLY if we broke at a space
        if end_pos < chars.len() && chars[end_pos] == ' ' {
            start_pos = end_pos + 1;
        } else {
            start_pos = end_pos;
        }
        
        is_first_line = false;
    }

    if result.is_empty() {
        result.push((String::new(), 0));
    }

    result
}

/// Detects the programming language from a file path based on its extension.
pub fn detect_language_from_path(path: &std::path::Path) -> String {
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        match extension {
            "rs" => "rust".to_string(),
            "py" => "python".to_string(),
            "js" | "jsx" => "javascript".to_string(),
            "ts" | "tsx" => "typescript".to_string(),
            "sh" | "bash" => "bash".to_string(),
            "json" => "json".to_string(),
            "toml" => "toml".to_string(),
            "sql" | "mysql" | "pgsql" | "sqlite" => "sql".to_string(),
            "html" | "htm" => "html".to_string(),
            "css" => "css".to_string(),
            "md" | "markdown" => "markdown".to_string(),
            "yaml" | "yml" => "yaml".to_string(),
            "xml" => "xml".to_string(),
            "c" => "c".to_string(),
            "cpp" | "cc" | "cxx" => "cpp".to_string(),
            "h" | "hpp" => "c".to_string(),
            "go" => "go".to_string(),
            "java" => "java".to_string(),
            "php" => "php".to_string(),
            "rb" => "ruby".to_string(),
            "swift" => "swift".to_string(),
            "kt" => "kotlin".to_string(),
            "scala" => "scala".to_string(),
            "clj" | "cljs" => "clojure".to_string(),
            "hs" => "haskell".to_string(),
            "elm" => "elm".to_string(),
            "ex" | "exs" => "elixir".to_string(),
            "erl" => "erlang".to_string(),
            "lua" => "lua".to_string(),
            "pl" | "pm" => "perl".to_string(),
            "r" => "r".to_string(),
            "dart" => "dart".to_string(),
            "vim" => "vim".to_string(),
            "dockerfile" => "dockerfile".to_string(),
            "makefile" => "makefile".to_string(),
            _ => "text".to_string(),
        }
    } else {
        // Check for special filenames
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            match filename.to_lowercase().as_str() {
                "dockerfile" => "dockerfile".to_string(),
                "makefile" => "makefile".to_string(),
                "gemfile" => "ruby".to_string(),
                "rakefile" => "ruby".to_string(),
                _ => "text".to_string(),
            }
        } else {
            "text".to_string()
        }
    }
}

/// Gets the display name for a programming language.
pub fn get_language_display_name(language: &str) -> &'static str {
    match language {
        "text" => "Plain Text",
        "rust" => "Rust",
        "python" => "Python",
        "javascript" => "JavaScript",
        "typescript" => "TypeScript",
        "bash" => "Bash/Shell",
        "json" => "JSON",
        "toml" => "TOML",
        "sql" => "SQL",
        "html" => "HTML",
        "css" => "CSS",
        "markdown" => "Markdown",
        "yaml" => "YAML",
        "xml" => "XML",
        "c" => "C",
        "cpp" => "C++",
        "go" => "Go",
        "java" => "Java",
        "php" => "PHP",
        "ruby" => "Ruby",
        "swift" => "Swift",
        "kotlin" => "Kotlin",
        "scala" => "Scala",
        "clojure" => "Clojure",
        "haskell" => "Haskell",
        "elm" => "Elm",
        "elixir" => "Elixir",
        "erlang" => "Erlang",
        "lua" => "Lua",
        "perl" => "Perl",
        "r" => "R",
        "dart" => "Dart",
        "vim" => "Vim Script",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        _ => "Unknown",
    }
}

/// Gets the list of supported languages.
pub fn get_supported_languages() -> Vec<&'static str> {
    vec![
        "text",
        "rust",
        "python",
        "javascript",
        "typescript",
        "bash",
        "json",
        "toml",
        "sql",
        "html",
        "css",
        "markdown",
        "yaml",
        "xml",
        "c",
        "cpp",
        "go",
        "java",
        "php",
        "ruby",
        "swift",
        "kotlin",
        "scala",
        "clojure",
        "haskell",
        "elm",
        "elixir",
        "erlang",
        "lua",
        "perl",
        "r",
        "dart",
        "vim",
        "dockerfile",
        "makefile",
    ]
}
