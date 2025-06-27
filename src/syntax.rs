// src/syntax.rs
//
// Tree-sitter syntax highlighting with a single tree-sitter version (0.20.10)
// SQL support is provided by compiling tree-sitter-sql from source

use ropey::Rope;
use std::collections::HashMap;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree, QueryCapture};

// External function to get the SQL language (compiled from source)
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

        self.language = language.to_string();
        
        let ts_language = match language {
            "rust" => Some(tree_sitter_rust::language()),
            "python" => Some(tree_sitter_python::language()),
            "javascript" | "typescript" => Some(tree_sitter_javascript::language()),
            "bash" => Some(tree_sitter_bash::language()),
            "json" => Some(tree_sitter_json::language()),
            "toml" => Some(tree_sitter_toml::language()),
            "sql" => {
                // Try to load the compiled SQL parser
                #[cfg(not(feature = "no-sql"))]
                {
                    Some(unsafe { tree_sitter_sql() })
                }
                #[cfg(feature = "no-sql")]
                {
                    None
                }
            }
            _ => None,
        };

        if let Some(lang) = ts_language {
            self.parser = Parser::new();
            if self.parser.set_language(lang).is_ok() {
                self.query = Self::create_highlight_query(language, lang);
                self.tree = None;
                self.needs_update = true;
            }
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
                (line_comment) @comment
                (block_comment) @comment
                (string_literal) @string
                (raw_string_literal) @string
                (char_literal) @string
                (integer_literal) @number
                (float_literal) @number
                (boolean_literal) @constant
                
                [
                    "use" "mod" "pub" "crate" "super" "self"
                    "fn" "let" "mut" "const" "static"
                    "if" "else" "match" "for" "while" "loop"
                    "break" "continue" "return"
                    "struct" "enum" "impl" "trait"
                    "async" "await" "unsafe" "extern"
                    "where" "as" "ref" "move"
                    "dyn" "type" "in"
                ] @keyword
                
                (type_identifier) @type
                (primitive_type) @type
                (function_item name: (identifier) @function)
                (call_expression function: (identifier) @function)
                (call_expression function: (field_expression field: (field_identifier) @function))
                (macro_invocation macro: (identifier) @function)
                
                (field_identifier) @property
                (identifier) @variable
                
                ["(" ")" "[" "]" "{" "}"] @punctuation
                ["+" "-" "*" "/" "%" "=" "==" "!=" "<" ">" "<=" ">=" "&&" "||" "!" "&" "|" "^" "<<" ">>"] @operator
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
                    "def" "class" "return" "yield" "raise" "assert"
                    "if" "elif" "else" "for" "while" "break" "continue"
                    "try" "except" "finally" "with" "as"
                    "import" "from" "global" "nonlocal"
                    "lambda" "pass" "del" "and" "or" "not" "in" "is"
                ] @keyword
                
                (function_definition name: (identifier) @function)
                (call function: (identifier) @function)
                (class_definition name: (identifier) @type)
                
                (attribute attribute: (identifier) @property)
                (identifier) @variable
                
                ["(" ")" "[" "]" "{" "}"] @punctuation
                ["+" "-" "*" "/" "//" "%" "**" "=" "==" "!=" "<" ">" "<=" ">=" "&" "|" "^" "~" "<<" ">>"] @operator
            "#,
            
            "javascript" | "typescript" => r#"
                (comment) @comment
                (string) @string
                (template_string) @string
                (number) @number
                (true) @constant
                (false) @constant
                (null) @constant
                (undefined) @constant
                
                [
                    "var" "let" "const" "function" "return"
                    "if" "else" "for" "while" "do" "break" "continue"
                    "switch" "case" "default" "try" "catch" "finally"
                    "throw" "new" "delete" "typeof" "instanceof"
                    "class" "extends" "super" "static"
                    "import" "export" "from" "as" "default"
                    "async" "await" "yield"
                ] @keyword
                
                (function_declaration name: (identifier) @function)
                (method_definition name: (property_identifier) @function)
                (call_expression function: (identifier) @function)
                
                (property_identifier) @property
                (identifier) @variable
                
                ["(" ")" "[" "]" "{" "}"] @punctuation
                ["+" "-" "*" "/" "%" "=" "==" "===" "!=" "!==" "<" ">" "<=" ">=" "&&" "||" "!" "&" "|" "^" "<<" ">>"] @operator
            "#,
            
            "bash" => r#"
                (comment) @comment
                (string) @string
                (raw_string) @string
                (number) @number
                
                [
                    "if" "then" "else" "elif" "fi"
                    "for" "while" "do" "done" "break" "continue"
                    "case" "esac" "in" "function" "return"
                    "local" "export" "declare" "readonly"
                    "test" "echo" "printf" "read"
                ] @keyword
                
                (command_name) @function
                (variable_name) @variable
                
                ["(" ")" "[" "]" "{" "}" "|" "&" ";" "&&" "||"] @punctuation
                ["=" "==" "!=" "-eq" "-ne" "-lt" "-le" "-gt" "-ge"] @operator
            "#,
            
            "sql" => r#"
                (comment) @comment
                (string) @string
                (number) @number
                
                [
                    "SELECT" "FROM" "WHERE" "JOIN" "INNER" "LEFT" "RIGHT" "FULL" "OUTER"
                    "ON" "GROUP" "BY" "HAVING" "ORDER" "ASC" "DESC" "LIMIT" "OFFSET"
                    "INSERT" "INTO" "VALUES" "UPDATE" "SET" "DELETE"
                    "CREATE" "TABLE" "ALTER" "DROP" "INDEX"
                    "PRIMARY" "KEY" "FOREIGN" "REFERENCES"
                    "AND" "OR" "NOT" "IN" "LIKE" "BETWEEN" "IS" "NULL"
                    "DISTINCT" "CASE" "WHEN" "THEN" "ELSE" "END"
                    "UNION" "ALL" "AS" "WITH" "RECURSIVE"
                    "QUALIFY" "WINDOW" "OVER" "PARTITION"
                    "VARIANT" "OBJECT" "ARRAY" "INTEGER" "VARCHAR" "DATE"
                    "TIMESTAMP" "BOOLEAN" "FLOAT" "DOUBLE" "NUMBER"
                ] @keyword
                
                (function_call name: (identifier) @function)
                (identifier) @variable
                
                ["(" ")" "," ";" "."] @punctuation
                ["=" "!=" "<" ">" "<=" ">=" "+" "-" "*" "/" "%" "||"] @operator
            "#,
            
            "json" => r#"
                (string) @string
                (number) @number
                (true) @constant
                (false) @constant
                (null) @constant
                
                (pair key: (string) @property)
                
                ["{" "}" "[" "]" ":" ","] @punctuation
            "#,
            
            "toml" => r#"
                (comment) @comment
                (string) @string
                (integer) @number
                (float) @number
                (boolean) @constant
                
                (bare_key) @property
                (quoted_key) @property
                
                ["[" "]" "=" "." ","] @punctuation
            "#,
            
            _ => return None,
        };

        Query::new(ts_language, query_string).ok()
    }

    pub fn update(&mut self, rope: &Rope) {
        if !self.needs_update {
            return;
        }

        let text = rope.to_string();
        
        if let Some(new_tree) = self.parser.parse(&text, self.tree.as_ref()) {
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
                    
                    // Convert byte positions to character positions
                    let start_char = text[..start_byte].chars().count();
                    let end_char = start_char + text[start_byte..end_byte].chars().count();
                    
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
                        "punctuation" => TokenType::Punctuation,
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
    }

    pub fn force_update(&mut self) {
        self.needs_update = true;
    }
}
