// src/syntax.rs
//
// Tree-sitter syntax highlighting with a single tree-sitter version (0.20.10)
// SQL support is provided by compiling tree-sitter-sql from source

use ropey::Rope;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

// External function to get the SQL language (compiled from source)
#[cfg(feature = "sql")]
extern "C" { fn tree_sitter_sql() -> Language; }

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
    parser: Parser,
    tree: Option<Tree>,
    query: Option<Query>,
    tokens_by_line: HashMap<usize, Vec<SyntaxToken>>,
    needs_update: bool,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            language: "text".to_string(),
            parser: Parser::new(),
            tree: None,
            query: None,
            tokens_by_line: HashMap::new(),
            needs_update: true,
        }
    }

    pub fn set_language(&mut self, language: &str) {
        if self.language == language {
            return;
        }

        match File::create("/tmp/thyme_debug.log") {
            Ok(mut file) => {
                writeln!(file, "[DEBUG] Setting language from '{}' to '{}'\n",
                    self.language, language).expect("Failed to write to debug log");
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to create debug log: {}", e);
            }
        }

        self.language = language.to_string();
        self.tokens_by_line.clear();
        self.tree = None;
        
        let ts_language = match language {
            "rust" => Some(tree_sitter_rust::language()),
            "python" => Some(tree_sitter_python::language()),
            "javascript" | "typescript" => Some(tree_sitter_javascript::language()),
            "bash" => Some(tree_sitter_bash::language()),
            "json" => Some(tree_sitter_json::language()),
            "toml" => Some(tree_sitter_toml::language()),
            "sql" => {
                #[cfg(feature = "sql")]
                {
                    Some(unsafe { tree_sitter_sql() })
                }
                #[cfg(not(feature = "sql"))]
                {
                    None
                }
            }
            _ => None,
        };

        if let Some(lang) = ts_language {
            self.parser = Parser::new();
            if let Err(_) = self.parser.set_language(lang) {
                self.query = None;
                self.tree = None;
                return;
            }
            
            match Self::create_highlight_query(language, lang) {
                Some(query) => {
                    self.query = Some(query);
                },
                None => {
                    self.query = None;
                }
            }
            self.tree = None;
            self.needs_update = true;
        } else {
            // Fallback to no highlighting for unsupported languages
            self.parser = Parser::new();
            self.query = None;
            self.tree = None;
        }
    }

    fn create_highlight_query(language: &str, ts_language: Language) -> Option<Query> {
        let query_string = match language {
            "rust" => r#"
                "fn" @keyword
                "let" @keyword
                (identifier) @variable
            "#,
            
            "python" => r#"
                (comment) @comment
                (string) @string
                (integer) @number
                (float) @number
                (true) @constant
                (false) @constant
                (none) @constant
                
                [
                    "and" "as" "assert" "async" "await" "break" "class" "continue"
                    "def" "del" "elif" "else" "except" "exec" "finally" "for" "from"
                    "global" "if" "import" "in" "is" "lambda" "nonlocal" "not" "or"
                    "pass" "print" "raise" "return" "try" "while" "with" "yield"
                ] @keyword
                
                (function_definition name: (identifier) @function)
                (call function: (identifier) @function)
                (call function: (attribute attribute: (identifier) @function))
                (class_definition name: (identifier) @type)
                
                (attribute attribute: (identifier) @property)
                (identifier) @variable
                
                ["(" ")" "[" "]" "{" "}"] @punctuation.bracket
                ["." "," ":" ";"] @punctuation.delimiter
                [
                    "+" "-" "*" "/" "//" "%" "**"
                    "==" "!=" "<" ">" "<=" ">=" 
                    "=" "+=" "-=" "*=" "/=" "//=" "%=" "**="
                    "&" "|" "^" "~" "<<" ">>"
                    "@"
                ] @operator
            "#,
            
            "javascript" | "typescript" => r#"
                (comment) @comment
                (string) @string
                (template_string) @string
                (regex) @string
                (number) @number
                (true) @constant
                (false) @constant
                (null) @constant
                (undefined) @constant
                
                [
                    "async" "await" "break" "case" "catch" "class" "const" "continue"
                    "debugger" "default" "delete" "do" "else" "export" "extends"
                    "finally" "for" "from" "function" "get" "if" "import" "in"
                    "instanceof" "let" "new" "of" "return" "set" "static" "switch"
                    "target" "throw" "try" "typeof" "var" "void" "while" "with"
                    "yield"
                ] @keyword
                
                (function_declaration name: (identifier) @function)
                (function name: (identifier) @function)
                (method_definition name: (property_identifier) @function)
                (call_expression function: (identifier) @function)
                (call_expression 
                    function: (member_expression 
                        property: (property_identifier) @function))
                
                (property_identifier) @property
                (shorthand_property_identifier) @property
                (identifier) @variable
                
                ["(" ")" "[" "]" "{" "}"] @punctuation.bracket
                [";" "." "," ":"] @punctuation.delimiter
                [
                    "+" "-" "*" "/" "%" "**"
                    "=" "+=" "-=" "*=" "/=" "%=" "**="
                    "==" "===" "!=" "!==" "<" ">" "<=" ">="
                    "&&" "||" "!" "??" 
                    "&" "|" "^" "~" "<<" ">>" ">>>"
                    "++""--" "..." "?." "=>" 
                ] @operator
            "#,
            
            "bash" => r#"
                (comment) @comment
                (string) @string
                (raw_string) @string
                (ansi_c_string) @string
                (number) @number
                
                [
                    "if" "then" "else" "elif" "fi"
                    "case" "esac" "for" "while" "until"
                    "do" "done" "select" "in"
                    "function" "return" "break" "continue"
                    "local" "readonly" "unset"
                    "export" "declare" "typeset"
                    "source" "alias" "unalias"
                ] @keyword
                
                (function_definition name: (word) @function)
                (command_name (word) @function)
                
                (variable_name) @variable
                ((word) @constant
                    (#match? @constant "^[A-Z_]+$"))
                
                "$" @punctuation.special
                ["(" ")" "[" "]" "{" "}" "[[" "]]"] @punctuation.bracket
                [";" "&" "|" "||" "&&" ";;" ";&" ";;&"] @punctuation.delimiter
                ["=" "+=" "-=" "*=" "/=" "%=" "**=" "&=" "|=" "^=" 
                 "<<=" ">>=" "==" "!=" "<" ">" "-eq" "-ne" "-lt" 
                 "-le" "-gt" "-ge"] @operator
            "#,
            
            "sql" => r#"
                (comment) @comment
                (marginalia) @comment
                
                (literal) @string
                
                ((literal) @number
                 (#match? @number "^[-+]?\d+$"))
                
                ((literal) @float
                 (#match? @float "^[-+]?\d*\.\d*$"))
                
                [
                    (keyword_select) (keyword_from) (keyword_where) (keyword_and) (keyword_or)
                    (keyword_not) (keyword_in) (keyword_like) (keyword_is) (keyword_null)
                    (keyword_order) (keyword_by) (keyword_group) (keyword_having) (keyword_limit)
                    (keyword_offset) (keyword_distinct) (keyword_as) (keyword_join) (keyword_left)
                    (keyword_right) (keyword_inner) (keyword_outer) (keyword_full) (keyword_cross)
                    (keyword_on) (keyword_union) (keyword_intersect) (keyword_except)
                    (keyword_insert) (keyword_into) (keyword_values) (keyword_update) (keyword_set)
                    (keyword_delete) (keyword_create) (keyword_table) (keyword_alter) (keyword_drop)
                    (keyword_primary) (keyword_key) (keyword_foreign) (keyword_references)
                    (keyword_constraint) (keyword_unique) (keyword_check) (keyword_default)
                    (keyword_index) (keyword_view) (keyword_database) (keyword_schema)
                    (keyword_if) (keyword_exists) (keyword_cascade) (keyword_restrict)
                    (keyword_case) (keyword_when) (keyword_then) (keyword_else) (keyword_end)
                    (keyword_begin) (keyword_commit) (keyword_rollback) (keyword_transaction)
                ] @keyword
                
                [
                    (keyword_int) (keyword_varchar) (keyword_char) (keyword_text) (keyword_boolean)
                    (keyword_date) (keyword_time) (keyword_timestamp) (keyword_decimal) (keyword_float)
                    (keyword_double) (keyword_numeric) (keyword_bigint) (keyword_smallint)
                    (keyword_tinyint) (keyword_mediumint) (keyword_real) (keyword_binary)
                    (keyword_varbinary) (keyword_json) (keyword_uuid)
                ] @type
                
                [
                    (keyword_true) (keyword_false)
                ] @constant
                
                (field name: (identifier) @property)
                (object_reference name: (identifier) @type)
                (relation alias: (identifier) @variable)
                (term alias: (identifier) @variable)
                
                (invocation (object_reference name: (identifier) @function))
                
                (all_fields) @operator
                
                ["+" "-" "/" "%" "^" ":=" "=" "<" "<=" "!=" ">=" ">" "<>"] @operator
                ["(" ")"] @punctuation.bracket
                [";" "," "."] @punctuation.delimiter
            "#,
            
            "json" => r#"
                (string_content) @string
                (number) @number
                (true) @constant
                (false) @constant
                (null) @constant
                
                (pair key: (string (string_content) @property))
                
                ["{" "}" "[" "]"] @punctuation.bracket
                [":" ","] @punctuation.delimiter
            "#,
            
            "toml" => r#"
                (comment) @comment
                (string) @string
                (integer) @number
                (float) @number
                (boolean) @constant
                (local_date) @string
                (local_date_time) @string
                (local_time) @string
                
                (bare_key) @property
                (quoted_key) @property
                
                ["[" "]" "[[" "]]"] @punctuation.bracket
                ["=" "." ","] @punctuation.delimiter
            "#,
            
            _ => return None,
        };

        Query::new(ts_language, query_string).ok()
    }

    pub fn update(&mut self, rope: &Rope) {
        // Always clear tokens when updating to ensure fresh highlighting
        self.tokens_by_line.clear();

        // Handle empty rope to prevent panics
        if rope.len_chars() == 0 {
            self.tree = None;
            self.needs_update = false;
            return;
        }

        let text = rope.to_string();
        match File::create("/tmp/thyme_debug.log") {
            Ok(mut file) => {
                writeln!(file, "[DEBUG] Parsing language: {}, text_len: {}\n",
                    self.language, text.len()).expect("Failed to write to debug log");
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to create debug log: {}", e);
            }
        }
        
        // Always re-parse the entire text for consistent highlighting
        if let Some(new_tree) = self.parser.parse(&text, None) {
            self.tree = Some(new_tree);
            self.highlight_tree(rope);
        }
        
        self.needs_update = false;
    }

    fn highlight_tree(&mut self, rope: &Rope) {
        self.tokens_by_line.clear();

        // Clone what we need to avoid borrow issues
        let (tree_exists, query_exists) = (self.tree.is_some(), self.query.is_some());
        if !tree_exists || !query_exists {
            return;
        }

        let text = rope.to_string();
        let text_bytes = text.as_bytes();

        // Collect all tokens in a separate scope to avoid borrow conflicts
        let tokens_to_add = {
            let tree = self.tree.as_ref().unwrap();
            let query = self.query.as_ref().unwrap();
            
            let mut query_cursor = QueryCursor::new();
            let mut tokens = Vec::new();
            let matches = query_cursor.matches(query, tree.root_node(), text_bytes);

            for match_ in matches {
                for capture in match_.captures {
                    let node = capture.node;
                    let start_byte = node.start_byte();
                    let end_byte = node.end_byte();
                    
                    // Convert byte positions to character positions safely
                    let start_char = self.safe_byte_to_char(rope, start_byte);
                    let end_char = self.safe_byte_to_char(rope, end_byte);
                    
                    // Debug: skip invalid tokens
                    if start_char >= end_char || start_char >= rope.len_chars() {
                        continue;
                    }
                    
                    // Determine token type from capture name
                    let capture_name = &query.capture_names()[capture.index as usize];
                    let token_type = match capture_name.as_str() {
                        "keyword" => TokenType::Keyword,
                        "string" => TokenType::String,
                        "comment" => TokenType::Comment,
                        "number" => TokenType::Number,
                        "operator" => TokenType::Operator,
                        "function" => TokenType::Function,
                        "type" => TokenType::Type,
                        "variable" => TokenType::Variable,
                        "property" => TokenType::Property,
                        "parameter" => TokenType::Parameter,
                        "constant" => TokenType::Constant,
                        "namespace" => TokenType::Namespace,
                        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => TokenType::Punctuation,
                        "tag" => TokenType::Tag,
                        "attribute" => TokenType::Attribute,
                        _ => TokenType::Normal,
                    };
                    
                    tokens.push((token_type, start_char, end_char));
                }
            }
            
            tokens
        }; // Scope ends here, releasing borrows

        // Now add all tokens to lines
        for (token_type, start_char, end_char) in tokens_to_add {
            self.add_token_to_lines(token_type, start_char, end_char, rope);
        }

        // Sort tokens by start position for each line
        for tokens in self.tokens_by_line.values_mut() {
            tokens.sort_by_key(|t| t.start);
        }
    }

    fn add_token_to_lines(&mut self, token_type: TokenType, start_char: usize, end_char: usize, rope: &Rope) {
        // Find which lines this token spans
        let start_line = rope.char_to_line(start_char);
        let end_line = rope.char_to_line(end_char.saturating_sub(1));

        for line in start_line..=end_line {
            let line_start_char = rope.line_to_char(line);
            let line_end_char = if line + 1 < rope.len_lines() {
                rope.line_to_char(line + 1).saturating_sub(1)
            } else {
                rope.len_chars()
            };
            
            // Calculate token position relative to line start
            let token_start_in_line = if start_char >= line_start_char {
                start_char - line_start_char
            } else {
                0
            };
            
            let token_end_in_line = if end_char <= line_end_char {
                end_char - line_start_char
            } else {
                line_end_char - line_start_char
            };

            if token_start_in_line < token_end_in_line {
                let token = SyntaxToken {
                    token_type: token_type.clone(),
                    start: token_start_in_line,
                    end: token_end_in_line,
                };

                self.tokens_by_line
                    .entry(line)
                    .or_insert_with(Vec::new)
                    .push(token);
            }
        }
    }

    pub fn get_line_tokens(&self, line: usize) -> Option<&Vec<SyntaxToken>> {
        self.tokens_by_line.get(&line)
    }

    pub fn mark_dirty(&mut self) {
        self.needs_update = true;
        // Clear existing tokens to force fresh highlighting
        self.tokens_by_line.clear();
    }

    pub fn force_update(&mut self) {
        self.needs_update = true;
    }
    
    fn safe_byte_to_char(&self, rope: &Rope, byte_pos: usize) -> usize {
        // Try to use rope's byte_to_char if available, otherwise fall back to manual conversion
        // For ropey 1.6, byte_to_char should be available
        rope.byte_to_char(byte_pos.min(rope.len_bytes()))
    }
}
