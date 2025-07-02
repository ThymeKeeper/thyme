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

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "//", &mut tokens);
        self.highlight_numbers(line, &mut tokens);

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

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);
        self.highlight_numbers(line, &mut tokens);

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

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "//", &mut tokens);
        self.highlight_numbers(line, &mut tokens);

        tokens
    }

    fn highlight_bash(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = [
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until",
            "do", "done", "function", "return", "break", "continue", "local", "export",
            "readonly", "declare", "unset", "source", "alias",
        ];

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);

        tokens
    }

    fn highlight_json(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = ["true", "false", "null"];

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_numbers(line, &mut tokens);

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

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "--", &mut tokens);
        self.highlight_numbers(line, &mut tokens);

        tokens
    }

    fn highlight_toml(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let keywords = ["true", "false"];

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments(line, "#", &mut tokens);
        self.highlight_numbers(line, &mut tokens);

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

        self.highlight_keywords(line, &keywords, &mut tokens);
        self.highlight_strings(line, &mut tokens);
        self.highlight_comments_range(line, "/*", "*/", &mut tokens);

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
                
                // Check word boundaries
                let is_word_start = keyword_start == 0 || 
                    !line.chars().nth(keyword_start - 1).unwrap_or(' ').is_alphanumeric();
                let is_word_end = keyword_end >= line.len() || 
                    !line.chars().nth(keyword_end).unwrap_or(' ').is_alphanumeric();
                
                if is_word_start && is_word_end {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Keyword,
                        start: keyword_start,
                        end: keyword_end,
                    });
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
                    tokens.push(SyntaxToken {
                        token_type: TokenType::String,
                        start: string_start,
                        end: i + ch.len_utf8(),
                    });
                    in_string = false;
                }
            }
        }
    }

    fn highlight_comments(&self, line: &str, comment_prefix: &str, tokens: &mut Vec<SyntaxToken>) {
        if let Some(pos) = line.find(comment_prefix) {
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                start: pos,
                end: line.len(),
            });
        }
    }

    fn highlight_comments_range(&self, line: &str, start_comment: &str, end_comment: &str, tokens: &mut Vec<SyntaxToken>) {
        let mut start = 0;
        while let Some(comment_start) = line[start..].find(start_comment) {
            let absolute_start = start + comment_start;
            if let Some(comment_end_rel) = line[absolute_start + start_comment.len()..].find(end_comment) {
                let absolute_end = absolute_start + start_comment.len() + comment_end_rel + end_comment.len();
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    start: absolute_start,
                    end: absolute_end,
                });
                start = absolute_end;
            } else {
                // Comment extends to end of line
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    start: absolute_start,
                    end: line.len(),
                });
                break;
            }
        }
    }

    fn highlight_numbers(&self, line: &str, tokens: &mut Vec<SyntaxToken>) {
        let mut chars = line.char_indices().peekable();
        
        while let Some((i, ch)) = chars.next() {
            if ch.is_ascii_digit() {
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
                
                tokens.push(SyntaxToken {
                    token_type: TokenType::Number,
                    start,
                    end,
                });
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
}
