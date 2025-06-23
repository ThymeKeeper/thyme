// src/syntax.rs

use regex::Regex;
use ropey::Rope;
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
    Normal,
}

#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct LineState {
    pub in_multiline_comment: bool,
    pub in_multiline_string: bool,
    pub string_delimiter: Option<char>,
}

impl Default for LineState {
    fn default() -> Self {
        Self {
            in_multiline_comment: false,
            in_multiline_string: false,
            string_delimiter: None,
        }
    }
}

pub struct SyntaxHighlighter {
    language: String,
    keywords: HashMap<String, TokenType>,
    patterns: Vec<(Regex, TokenType)>,
    line_states: Vec<LineState>,
    tokens_by_line: HashMap<usize, Vec<SyntaxToken>>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            language: "text".to_string(),
            keywords: HashMap::new(),
            patterns: Vec::new(),
            line_states: Vec::new(),
            tokens_by_line: HashMap::new(),
        }
    }

    pub fn set_language(&mut self, language: &str) {
        self.language = language.to_string();
        self.setup_language_rules();
    }

    fn setup_language_rules(&mut self) {
        self.keywords.clear();
        self.patterns.clear();

        match self.language.as_str() {
            "rust" => self.setup_rust_rules(),
            "python" => self.setup_python_rules(),
            "javascript" => self.setup_javascript_rules(),
            "sql" => self.setup_sql_rules(),
            "bash" => self.setup_bash_rules(),
            "xml" => self.setup_xml_rules(),
            _ => {}
        }
    }

    fn setup_rust_rules(&mut self) {
        let keywords = vec![
            "fn", "let", "mut", "const", "static", "if", "else", "match", "for", "while", "loop",
            "break", "continue", "return", "struct", "enum", "impl", "trait", "mod", "use", "pub",
            "crate", "super", "self", "Self", "where", "async", "await", "unsafe", "extern",
        ];

        for keyword in keywords {
            self.keywords.insert(keyword.to_string(), TokenType::Keyword);
        }

        let types = vec!["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result"];
        for type_name in types {
            self.keywords.insert(type_name.to_string(), TokenType::Type);
        }

        self.patterns = vec![
            (Regex::new(r#""([^"\\]|\\.)*""#).unwrap(), TokenType::String),
            (Regex::new(r"//.*$").unwrap(), TokenType::Comment),
            (Regex::new(r"/\*.*?\*/").unwrap(), TokenType::Comment),
            (Regex::new(r"\b\d+\b").unwrap(), TokenType::Number),
            (Regex::new(r"[+\-*/%=<>!&|^]").unwrap(), TokenType::Operator),
            (Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\s*\(").unwrap(), TokenType::Function),
        ];
    }

    fn setup_python_rules(&mut self) {
        let keywords = vec![
            "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally",
            "import", "from", "as", "return", "yield", "lambda", "with", "assert", "pass",
            "break", "continue", "global", "nonlocal", "and", "or", "not", "in", "is",
        ];

        for keyword in keywords {
            self.keywords.insert(keyword.to_string(), TokenType::Keyword);
        }

        self.patterns = vec![
            (Regex::new(r#""([^"\\]|\\.)*""#).unwrap(), TokenType::String),
            (Regex::new(r"'([^'\\]|\\.)*'").unwrap(), TokenType::String),
            (Regex::new(r"#.*$").unwrap(), TokenType::Comment),
            (Regex::new(r"\b\d+\b").unwrap(), TokenType::Number),
            (Regex::new(r"[+\-*/%=<>!&|^]").unwrap(), TokenType::Operator),
        ];
    }

    fn setup_javascript_rules(&mut self) {
        let keywords = vec![
            "function", "var", "let", "const", "if", "else", "for", "while", "do", "switch",
            "case", "default", "break", "continue", "return", "try", "catch", "finally",
            "throw", "new", "this", "typeof", "instanceof", "class", "extends", "import",
            "export", "async", "await",
        ];

        for keyword in keywords {
            self.keywords.insert(keyword.to_string(), TokenType::Keyword);
        }

        self.patterns = vec![
            (Regex::new(r#""([^"\\]|\\.)*""#).unwrap(), TokenType::String),
            (Regex::new(r"'([^'\\]|\\.)*'").unwrap(), TokenType::String),
            (Regex::new(r"//.*$").unwrap(), TokenType::Comment),
            (Regex::new(r"/\*.*?\*/").unwrap(), TokenType::Comment),
            (Regex::new(r"\b\d+\b").unwrap(), TokenType::Number),
            (Regex::new(r"[+\-*/%=<>!&|^]").unwrap(), TokenType::Operator),
        ];
    }

    fn setup_sql_rules(&mut self) {
        let keywords = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP",
            "TABLE", "INDEX", "VIEW", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON", "AS",
            "ORDER", "BY", "GROUP", "HAVING", "DISTINCT", "UNION", "AND", "OR", "NOT", "NULL",
        ];

        for keyword in keywords {
            self.keywords.insert(keyword.to_string(), TokenType::Keyword);
            self.keywords.insert(keyword.to_lowercase(), TokenType::Keyword);
        }

        self.patterns = vec![
            (Regex::new(r"'([^'\\]|\\.)*'").unwrap(), TokenType::String),
            (Regex::new(r"--.*$").unwrap(), TokenType::Comment),
            (Regex::new(r"\b\d+\b").unwrap(), TokenType::Number),
            (Regex::new(r"[=<>!]").unwrap(), TokenType::Operator),
        ];
    }

    fn setup_bash_rules(&mut self) {
        let keywords = vec![
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
            "function", "return", "local", "export", "declare", "readonly",
        ];

        for keyword in keywords {
            self.keywords.insert(keyword.to_string(), TokenType::Keyword);
        }

        self.patterns = vec![
            (Regex::new(r#""([^"\\]|\\.)*""#).unwrap(), TokenType::String),
            (Regex::new(r"'([^'\\]|\\.)*'").unwrap(), TokenType::String),
            (Regex::new(r"#.*$").unwrap(), TokenType::Comment),
            (Regex::new(r"\$\w+").unwrap(), TokenType::Identifier),
        ];
    }

    fn setup_xml_rules(&mut self) {
        self.patterns = vec![
            (Regex::new(r"<!--.*?-->").unwrap(), TokenType::Comment),
            (Regex::new(r#""([^"\\]|\\.)*""#).unwrap(), TokenType::String),
            (Regex::new(r"'([^'\\]|\\.)*'").unwrap(), TokenType::String),
            (Regex::new(r"<[^>]+>").unwrap(), TokenType::Keyword),
        ];
    }

    pub fn update(&mut self, rope: &Rope) {
        self.line_states.clear();
        self.tokens_by_line.clear();

        let mut current_state = LineState::default();

        for line_idx in 0..rope.len_lines() {
            let line_text = rope.line(line_idx).to_string();
            let tokens = self.highlight_line(&line_text, &current_state);
            
            // Update state for next line
            current_state = self.compute_line_end_state(&line_text, &current_state);
            self.line_states.push(current_state.clone());
            
            if !tokens.is_empty() {
                self.tokens_by_line.insert(line_idx, tokens);
            }
        }
    }

    fn highlight_line(&self, line: &str, start_state: &LineState) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let mut chars = line.char_indices().peekable();
        let mut current_state = start_state.clone();

        while let Some((pos, ch)) = chars.next() {
            // Handle multiline comments and strings
            if current_state.in_multiline_comment {
                if let Some(end_pos) = self.find_comment_end(line, pos) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Comment,
                        start: 0,
                        end: end_pos,
                    });
                    current_state.in_multiline_comment = false;
                    continue;
                } else {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Comment,
                        start: 0,
                        end: line.len(),
                    });
                    break;
                }
            }

            if current_state.in_multiline_string {
                if let Some(end_pos) = self.find_string_end(line, pos, current_state.string_delimiter.unwrap()) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::String,
                        start: 0,
                        end: end_pos,
                    });
                    current_state.in_multiline_string = false;
                    current_state.string_delimiter = None;
                    continue;
                } else {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::String,
                        start: 0,
                        end: line.len(),
                    });
                    break;
                }
            }

            // Apply regex patterns
            let remaining_line = &line[pos..];
            for (pattern, token_type) in &self.patterns {
                if let Some(mat) = pattern.find(remaining_line) {
                    if mat.start() == 0 {
                        tokens.push(SyntaxToken {
                            token_type: token_type.clone(),
                            start: pos,
                            end: pos + mat.end(),
                        });
                        
                        // Skip the matched characters
                        for _ in 0..mat.end() - 1 {
                            chars.next();
                        }
                        break;
                    }
                }
            }

            // Check for keywords
            if ch.is_alphabetic() || ch == '_' {
                let word_start = pos;
                let mut word_end = pos + ch.len_utf8();
                
                while let Some((next_pos, next_ch)) = chars.peek() {
                    if next_ch.is_alphanumeric() || *next_ch == '_' {
                        word_end = *next_pos + next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                
                let word = &line[word_start..word_end];
                if let Some(token_type) = self.keywords.get(word) {
                    tokens.push(SyntaxToken {
                        token_type: token_type.clone(),
                        start: word_start,
                        end: word_end,
                    });
                }
            }
        }

        tokens
    }

    fn compute_line_end_state(&self, line: &str, start_state: &LineState) -> LineState {
        let mut state = start_state.clone();
        
        // Simple state tracking - this would be more sophisticated in a real implementation
        if line.contains("/*") && !line.contains("*/") {
            state.in_multiline_comment = true;
        }
        if line.contains("*/") {
            state.in_multiline_comment = false;
        }
        
        state
    }

    fn find_comment_end(&self, line: &str, start: usize) -> Option<usize> {
        line[start..].find("*/").map(|pos| start + pos + 2)
    }

    fn find_string_end(&self, line: &str, start: usize, delimiter: char) -> Option<usize> {
        line[start..].find(delimiter).map(|pos| start + pos + 1)
    }

    pub fn get_line_tokens(&self, line: usize) -> Option<&Vec<SyntaxToken>> {
        self.tokens_by_line.get(&line)
    }
}
