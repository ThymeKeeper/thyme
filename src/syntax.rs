// src/syntax.rs
//
// Simple syntax highlighting without Tree-sitter dependency

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Keyword,
    String,
    Comment,
    Number,
    Operator,
    Identifier,
    Type,
    Function,
    Variable,
    Property,
    Parameter,
    Constant,
    Namespace,
    Punctuation,
    Tag,
    Attribute,
    Normal,
}

#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

pub struct SyntaxHighlighter {
    language: String,
    tokens_by_line: HashMap<usize, Vec<SyntaxToken>>,
    needs_update: bool,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            language: "text".to_string(),
            tokens_by_line: HashMap::new(),
            needs_update: true,
        }
    }

    pub fn set_language(&mut self, language: &str) {
        if self.language != language {
            self.language = language.to_string();
            self.tokens_by_line.clear();
            self.needs_update = true;
        }
    }

    pub fn update(&mut self, rope: &ropey::Rope) {
        self.tokens_by_line.clear();

        if rope.len_chars() == 0 || self.language == "text" {
            self.needs_update = false;
            return;
        }

        // Simple line-by-line highlighting for now
        for line_idx in 0..rope.len_lines() {
            let line_text = rope.line(line_idx).to_string();
            let tokens = self.highlight_line(&line_text, line_idx);
            if !tokens.is_empty() {
                self.tokens_by_line.insert(line_idx, tokens);
            }
        }

        self.needs_update = false;
    }

    fn highlight_line(&self, line: &str, _line_idx: usize) -> Vec<SyntaxToken> {
        match self.language.as_str() {
            "rust" => self.highlight_rust(line),
            "python" => self.highlight_python(line),
            "javascript" | "typescript" => self.highlight_javascript(line),
            "bash" => self.highlight_bash(line),
            "json" => self.highlight_json(line),
            "sql" => self.highlight_sql(line),
            "toml" => self.highlight_toml(line),
            "html" => self.highlight_html(line),
            "css" => self.highlight_css(line),
            "markdown" => self.highlight_markdown(line),
            _ => Vec::new(),
        }
    }

    fn highlight_rust(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "let", "mut", "fn", "if", "else", "while", "for", "loop", "match", "return",
            "struct", "enum", "impl", "trait", "use", "mod", "pub", "const", "static",
            "unsafe", "async", "await", "move", "ref", "self", "Self", "super", "crate",
            "where", "type", "as", "in", "break", "continue", "true", "false",
        ];
        
        let types = [
            "i8", "i16", "i32", "i64", "i128", "isize",
            "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str",
            "String", "Vec", "HashMap", "HashSet", "Option", "Result",
            "Box", "Rc", "Arc", "RefCell", "Mutex", "RwLock",
            "&str", "&mut", "std", "io", "fs", "path", "PathBuf",
        ];
        
        let operators = [
            "->", "=>", "::", "<=", ">=", "==", "!=", "&&", "||",
            "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "//", &mut tokens);
        self.highlight_rust_attributes(line, &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_rust_function_calls(line, &mut tokens);
        self.highlight_rust_function_definitions(line, &mut tokens);
        self.highlight_rust_types_and_generics(line, &mut tokens);
        self.highlight_rust_references(line, &mut tokens);
        self.highlight_rust_macros(line, &mut tokens);
        self.highlight_operators(line, &operators, &mut tokens);
        self.highlight_keywords(line, &types, &mut tokens); // Types as special keywords
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_python(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "def", "class", "if", "else", "elif", "while", "for", "in", "try", "except",
            "finally", "with", "as", "import", "from", "return", "yield", "pass", "break",
            "continue", "and", "or", "not", "is", "lambda", "global", "nonlocal", "True",
            "False", "None", "async", "await",
        ];
        
        let types = [
            "int", "float", "str", "bool", "list", "dict", "tuple", "set",
            "bytes", "object", "type", "Exception", "ValueError", "TypeError",
        ];
        
        let operators = [
            "==", "!=", ">=", "<=", "and", "or", "not", "in", "is",
            "+=", "-=", "*=", "/=", "//=", "%=", "**=",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_python_function_calls(line, &mut tokens);
        self.highlight_python_function_definitions(line, &mut tokens);
        self.highlight_operators(line, &operators, &mut tokens);
        self.highlight_keywords(line, &types, &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_javascript(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "function", "var", "let", "const", "if", "else", "while", "for", "do", "switch",
            "case", "default", "break", "continue", "return", "try", "catch", "finally",
            "throw", "new", "this", "typeof", "instanceof", "in", "of", "true", "false",
            "null", "undefined", "class", "extends", "super", "static", "async", "await",
            "import", "export", "from", "as",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "//", &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_bash(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until",
            "do", "done", "function", "return", "break", "continue", "local", "export",
            "readonly", "declare", "unset", "source", "alias",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_json(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = ["true", "false", "null"];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_sql(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "IS", "NULL",
            "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "DISTINCT", "AS", "JOIN",
            "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON", "UNION", "INTERSECT",
            "EXCEPT", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE",
            "TABLE", "ALTER", "DROP", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "INDEX",
            "VIEW", "DATABASE", "SCHEMA", "IF", "EXISTS", "CASCADE", "RESTRICT", "CASE",
            "WHEN", "THEN", "ELSE", "END", "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
            // Lowercase versions
            "select", "from", "where", "and", "or", "not", "in", "like", "is", "null",
            "order", "by", "group", "having", "limit", "offset", "distinct", "as", "join",
            "left", "right", "inner", "outer", "full", "cross", "on", "union", "intersect",
            "except", "insert", "into", "values", "update", "set", "delete", "create",
            "table", "alter", "drop", "primary", "key", "foreign", "references", "index",
            "view", "database", "schema", "if", "exists", "cascade", "restrict", "case",
            "when", "then", "else", "end", "begin", "commit", "rollback", "transaction",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "--", &mut tokens);
        self.highlight_comments_range(line, "/*", "*/", &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_toml(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = ["true", "false"];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);
        self.highlight_numbers(line, &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_html(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        // Simple HTML highlighting - look for tags
        let mut chars = line.char_indices().peekable();
        
        while let Some((i, ch)) = chars.next() {
            if ch == '<' {
                let start = i;
                let mut end = i + 1;
                let mut in_tag = true;
                
                while let Some((j, tag_ch)) = chars.next() {
                    end = j + tag_ch.len_utf8();
                    if tag_ch == '>' {
                        in_tag = false;
                        break;
                    }
                }
                
                if !in_tag {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Tag,
                        start,
                        end,
                    });
                }
            }
        }

        self.highlight_strings(line, &mut tokens);
        self.highlight_comments_range(line, "<!--", "-->", &mut tokens);

        tokens
    }

    fn highlight_css(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "color", "background", "font", "margin", "padding", "border", "width", "height",
            "display", "position", "float", "clear", "overflow", "text-align", "font-size",
            "font-weight", "line-height", "text-decoration",
        ];

        // Order matters: higher precedence tokens should be highlighted first
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments_range(line, "/*", "*/", &mut tokens);
        self.highlight_keywords(line, &keywords, &mut tokens);

        tokens
    }

    fn highlight_markdown(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        
        // Headers
        if line.starts_with('#') {
            let header_end = line.find(' ').unwrap_or(line.len());
            tokens.push(SyntaxToken {
                token_type: TokenType::Keyword,
                start: 0,
                end: header_end,
            });
        }
        
        // Code blocks
        if line.starts_with("```") {
            tokens.push(SyntaxToken {
                token_type: TokenType::String,
                start: 0,
                end: line.len(),
            });
        }

        tokens
    }

    fn highlight_keywords(&self, line: &str, keywords: &[&str], tokens: &mut Vec<SyntaxToken>) {
        for keyword in keywords {
            let mut start = 0;
            while let Some(pos) = line[start..].find(keyword) {
                let keyword_start = start + pos;
                let keyword_end = keyword_start + keyword.len();
                
                // Check word boundaries - more strict check for alphanumeric and underscore
                let is_word_start = keyword_start == 0 || 
                    !line.chars().nth(keyword_start - 1).unwrap_or(' ').is_alphanumeric() && 
                    line.chars().nth(keyword_start - 1).unwrap_or(' ') != '_';
                let is_word_end = keyword_end >= line.len() || 
                    (!line.chars().nth(keyword_end).unwrap_or(' ').is_alphanumeric() && 
                     line.chars().nth(keyword_end).unwrap_or(' ') != '_');
                
                if is_word_start && is_word_end {
                    // Check if this position is already covered by an existing token
                    let overlaps = tokens.iter().any(|existing_token| {
                        !(keyword_end <= existing_token.start || keyword_start >= existing_token.end)
                    });
                    
                    if !overlaps {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Keyword,
                            start: keyword_start,
                            end: keyword_end,
                        });
                    }
                }
                
                start = keyword_start + 1;
            }
        }
    }

    fn highlight_strings(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        self.highlight_string_with_quote(line, '"', tokens);
        self.highlight_string_with_quote(line, '\'', tokens);
    }

    fn highlight_string_with_quote(&self, line: &str, quote: char, tokens: &mut Vec<SyntaxToken>) {
        let mut chars = line.char_indices();
        let mut in_string = false;
        let mut string_start = 0;
        let mut escaped = false;

        while let Some((i, ch)) = chars.next() {
            if !in_string && ch == quote {
                in_string = true;
                string_start = i;
                escaped = false;
            } else if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == quote {
                    let string_end = i + ch.len_utf8();
                    
                    // Check if this position is already covered by an existing token
                    let overlaps = tokens.iter().any(|existing_token| {
                        !(string_end <= existing_token.start || string_start >= existing_token.end)
                    });
                    
                    if !overlaps {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::String,
                            start: string_start,
                            end: string_end,
                        });
                    }
                    in_string = false;
                }
            }
        }
    }

    fn highlight_comments(&self, line: &str, comment_prefix: &str, tokens: &mut Vec<SyntaxToken>) {
        if let Some(pos) = line.find(comment_prefix) {
            let comment_start = pos;
            let comment_end = line.len();
            
            // Check if this position is already covered by an existing token
            let overlaps = tokens.iter().any(|existing_token| {
                !(comment_end <= existing_token.start || comment_start >= existing_token.end)
            });
            
            if !overlaps {
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    start: comment_start,
                    end: comment_end,
                });
            }
        }
    }

    fn highlight_comments_range(&self, line: &str, start_comment: &str, end_comment: &str, tokens: &mut Vec<SyntaxToken>) {
        let mut start = 0;
        while let Some(comment_start) = line[start..].find(start_comment) {
            let absolute_start = start + comment_start;
            let absolute_end = if let Some(comment_end_rel) = line[absolute_start + start_comment.len()..].find(end_comment) {
                absolute_start + start_comment.len() + comment_end_rel + end_comment.len()
            } else {
                // Comment extends to end of line
                line.len()
            };
            
            // Check if this position is already covered by an existing token
            let overlaps = tokens.iter().any(|existing_token| {
                !(absolute_end <= existing_token.start || absolute_start >= existing_token.end)
            });
            
            if !overlaps {
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    start: absolute_start,
                    end: absolute_end,
                });
            }
            
            if absolute_end == line.len() {
                break;
            } else {
                start = absolute_end;
            }
        }
    }

    fn highlight_numbers(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        let mut chars = line.char_indices().peekable();
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_ascii_digit() {
                // Check if this digit is preceded by an alphanumeric character or underscore
                // If so, it's part of an identifier, not a standalone number
                let is_part_of_identifier = if i > 0 {
                    let prev_char = line.chars().nth(line[..i].chars().count().saturating_sub(1));
                    prev_char.map_or(false, |c| c.is_alphanumeric() || c == '_')
                } else {
                    false
                };
                
                if is_part_of_identifier {
                    continue;
                }
                
                let start = i;
                let mut end = i + ch.len_utf8();
                let mut has_dot = false;
                
                // Continue collecting digits and at most one decimal point
                while let Some(&(j, next_ch)) = chars.peek() {
                    if next_ch.is_ascii_digit() {
                        end = j + next_ch.len_utf8();
                        chars.next();
                    } else if next_ch == '.' && !has_dot {
                        // Look ahead to see if there's a digit after the dot
                        let mut temp_chars = chars.clone();
                        temp_chars.next(); // Skip the dot
                        if let Some((_, after_dot)) = temp_chars.next() {
                            if after_dot.is_ascii_digit() {
                                has_dot = true;
                                end = j + next_ch.len_utf8();
                                chars.next();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                
                // Also check if the number is followed by an alphanumeric character or underscore
                // If so, it's part of an identifier
                let is_followed_by_identifier = if end < line.len() {
                    let next_char = line.chars().nth(line[..end].chars().count());
                    next_char.map_or(false, |c| c.is_alphabetic() || c == '_')
                } else {
                    false
                };
                
                if is_followed_by_identifier {
                    continue;
                }
                
                // Check if this position is already covered by an existing token
                let overlaps = tokens.iter().any(|existing_token| {
                    !(end <= existing_token.start || start >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Number,
                        start,
                        end,
                    });
                }
            }
        }
    }

    pub fn get_line_tokens(&self, line: usize) -> Option<&Vec<SyntaxToken>> {
        self.tokens_by_line.get(&line)
    }

    pub fn mark_dirty(&mut self) {
        self.needs_update = true;
        self.tokens_by_line.clear();
    }

    pub fn force_update(&mut self) {
        self.needs_update = true;
    }
    
    // Rust-specific highlighting functions for comprehensive syntax coloring
    
    fn highlight_rust_function_calls(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match pattern: identifier followed by '('
        let mut chars = line.char_indices().peekable();
        let mut current_word_start = None;
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_alphabetic() || ch == '_' {
                if current_word_start.is_none() {
                    current_word_start = Some(i);
                }
            } else if ch.is_alphanumeric() {
                // Continue word
            } else {
                if let Some(word_start) = current_word_start {
                    // Check if next non-whitespace character is '('
                    let mut peek_chars = chars.clone();
                    let mut found_paren = false;
                    
                    if ch == '(' {
                        found_paren = true;
                    } else if ch.is_whitespace() {
                        // Skip whitespace to find '('
                        while let Some((_, peek_ch)) = peek_chars.next() {
                            if peek_ch == '(' {
                                found_paren = true;
                                break;
                            } else if !peek_ch.is_whitespace() {
                                break;
                            }
                        }
                    }
                    
                    if found_paren {
                        let overlaps = tokens.iter().any(|existing_token| {
                            !(i <= existing_token.start || word_start >= existing_token.end)
                        });
                        
                        if !overlaps {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Function,
                                start: word_start,
                                end: i,
                            });
                        }
                    }
                }
                current_word_start = None;
            }
        }
    }
    
    fn highlight_rust_function_definitions(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match pattern: 'fn' followed by identifier
        if let Some(fn_pos) = line.find("fn ") {
            let after_fn = fn_pos + 3;
            let remaining = &line[after_fn..];
            
            // Skip whitespace
            let mut start_pos = 0;
            for (i, ch) in remaining.char_indices() {
                if !ch.is_whitespace() {
                    start_pos = i;
                    break;
                }
            }
            
            // Find end of function name
            let mut end_pos = start_pos;
            for (i, ch) in remaining[start_pos..].char_indices() {
                if ch.is_alphanumeric() || ch == '_' {
                    end_pos = start_pos + i + ch.len_utf8();
                } else {
                    break;
                }
            }
            
            if end_pos > start_pos {
                let absolute_start = after_fn + start_pos;
                let absolute_end = after_fn + end_pos;
                
                let overlaps = tokens.iter().any(|existing_token| {
                    !(absolute_end <= existing_token.start || absolute_start >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Function,
                        start: absolute_start,
                        end: absolute_end,
                    });
                }
            }
        }
    }
    
    fn highlight_rust_types_and_generics(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match patterns like Vec<T>, Option<String>, HashMap<K, V>
        let mut chars = line.char_indices().peekable();
        let mut current_word_start = None;
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_ascii_uppercase() || (ch.is_alphabetic() && current_word_start.is_none()) {
                if current_word_start.is_none() {
                    current_word_start = Some(i);
                }
            } else if ch.is_alphanumeric() || ch == '_' {
                // Continue word
            } else {
                if let Some(word_start) = current_word_start {
                    // Check if this looks like a type (starts with uppercase or is a known type)
                    let word = &line[word_start..i];
                    let is_type = word.chars().next().unwrap_or('a').is_ascii_uppercase() ||
                                 matches!(word, "usize" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" |
                                              "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" |
                                              "bool" | "char" | "str");
                    
                    if is_type {
                        let overlaps = tokens.iter().any(|existing_token| {
                            !(i <= existing_token.start || word_start >= existing_token.end)
                        });
                        
                        if !overlaps {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Type,
                                start: word_start,
                                end: i,
                            });
                        }
                    }
                }
                current_word_start = None;
            }
        }
        
        // Handle end of line
        if let Some(word_start) = current_word_start {
            let word = &line[word_start..];
            let is_type = word.chars().next().unwrap_or('a').is_ascii_uppercase() ||
                         matches!(word, "usize" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" |
                                        "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" |
                                        "bool" | "char" | "str");
            
            if is_type {
                let overlaps = tokens.iter().any(|existing_token| {
                    !(line.len() <= existing_token.start || word_start >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Type,
                        start: word_start,
                        end: line.len(),
                    });
                }
            }
        }
    }
    
    fn highlight_rust_references(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match &, &mut, and reference patterns
        let mut chars = line.char_indices().peekable();
        
        while let Some((i, ch)) = chars.next() {
            if ch == '&' {
                let mut end = i + 1;
                
                // Check for &mut
                if line[i..].starts_with("&mut") {
                    end = i + 4;
                }
                
                let overlaps = tokens.iter().any(|existing_token| {
                    !(end <= existing_token.start || i >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Operator,
                        start: i,
                        end,
                    });
                }
            }
        }
    }
    
    fn highlight_rust_macros(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match patterns like println!, vec!, format!
        let mut chars = line.char_indices().peekable();
        let mut current_word_start = None;
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_alphabetic() || ch == '_' {
                if current_word_start.is_none() {
                    current_word_start = Some(i);
                }
            } else if ch.is_alphanumeric() {
                // Continue word
            } else {
                if let Some(word_start) = current_word_start {
                    if ch == '!' {
                        let overlaps = tokens.iter().any(|existing_token| {
                            !((i + 1) <= existing_token.start || word_start >= existing_token.end)
                        });
                        
                        if !overlaps {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Function,
                                start: word_start,
                                end: i + 1,
                            });
                        }
                    }
                }
                current_word_start = None;
            }
        }
    }
    
    fn highlight_rust_attributes(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match patterns like #[derive(Debug, Clone)], #[allow(dead_code)], etc.
        let mut chars = line.char_indices().peekable();
        
        while let Some((i, ch)) = chars.next() {
            if ch == '#' {
                // Look for opening bracket
                if let Some(&(j, '[')) = chars.peek() {
                    chars.next(); // consume the '['
                    
                    // Find the matching closing bracket
                    let mut bracket_count = 1;
                    let mut end_pos = j + 1;
                    
                    while let Some((k, bracket_ch)) = chars.next() {
                        end_pos = k + bracket_ch.len_utf8();
                        
                        if bracket_ch == '[' {
                            bracket_count += 1;
                        } else if bracket_ch == ']' {
                            bracket_count -= 1;
                            if bracket_count == 0 {
                                break;
                            }
                        }
                    }
                    
                    // Only highlight if we found a complete attribute
                    if bracket_count == 0 {
                        let overlaps = tokens.iter().any(|existing_token| {
                            !(end_pos <= existing_token.start || i >= existing_token.end)
                        });
                        
                        if !overlaps {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Attribute,
                                start: i,
                                end: end_pos,
                            });
                        }
                    }
                }
            }
        }
    }
    
    fn highlight_operators(&self, line: &str, operators: &[&str], tokens: &mut Vec<SyntaxToken>) {
        for operator in operators {
            let mut start = 0;
            while let Some(pos) = line[start..].find(operator) {
                let op_start = start + pos;
                let op_end = op_start + operator.len();
                
                let overlaps = tokens.iter().any(|existing_token| {
                    !(op_end <= existing_token.start || op_start >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Operator,
                        start: op_start,
                        end: op_end,
                    });
                }
                
                start = op_start + 1;
            }
        }
    }
    
    // Python-specific highlighting functions
    
    fn highlight_python_function_calls(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match pattern: identifier followed by '('
        let mut chars = line.char_indices().peekable();
        let mut current_word_start = None;
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_alphabetic() || ch == '_' {
                if current_word_start.is_none() {
                    current_word_start = Some(i);
                }
            } else if ch.is_alphanumeric() {
                // Continue word
            } else {
                if let Some(word_start) = current_word_start {
                    // Check if next non-whitespace character is '('
                    let mut peek_chars = chars.clone();
                    let mut found_paren = false;
                    
                    if ch == '(' {
                        found_paren = true;
                    } else if ch.is_whitespace() {
                        // Skip whitespace to find '('
                        while let Some((_, peek_ch)) = peek_chars.next() {
                            if peek_ch == '(' {
                                found_paren = true;
                                break;
                            } else if !peek_ch.is_whitespace() {
                                break;
                            }
                        }
                    }
                    
                    if found_paren {
                        let overlaps = tokens.iter().any(|existing_token| {
                            !(i <= existing_token.start || word_start >= existing_token.end)
                        });
                        
                        if !overlaps {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Function,
                                start: word_start,
                                end: i,
                            });
                        }
                    }
                }
                current_word_start = None;
            }
        }
    }
    
    fn highlight_python_function_definitions(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        // Match pattern: 'def' followed by identifier
        if let Some(def_pos) = line.find("def ") {
            let after_def = def_pos + 4;
            let remaining = &line[after_def..];
            
            // Skip whitespace
            let mut start_pos = 0;
            for (i, ch) in remaining.char_indices() {
                if !ch.is_whitespace() {
                    start_pos = i;
                    break;
                }
            }
            
            // Find end of function name
            let mut end_pos = start_pos;
            for (i, ch) in remaining[start_pos..].char_indices() {
                if ch.is_alphanumeric() || ch == '_' {
                    end_pos = start_pos + i + ch.len_utf8();
                } else {
                    break;
                }
            }
            
            if end_pos > start_pos {
                let absolute_start = after_def + start_pos;
                let absolute_end = after_def + end_pos;
                
                let overlaps = tokens.iter().any(|existing_token| {
                    !(absolute_end <= existing_token.start || absolute_start >= existing_token.end)
                });
                
                if !overlaps {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Function,
                        start: absolute_start,
                        end: absolute_end,
                    });
                }
            }
        }
    }
}
