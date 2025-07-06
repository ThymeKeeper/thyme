// src/syntax.rs
//
// State machine-based syntax highlighting

use std::collections::HashMap;
use ropey::Rope;

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScanState {
    Normal,
    InString { quote: char, escaped: bool },
    InSingleLineComment,
    InMultiLineComment { depth: usize }, // For nested comments
    InRawString { delimiter_len: usize }, // For Rust raw strings
}

#[derive(Debug, Clone)]
struct LineState {
    tokens: Vec<SyntaxToken>,
    end_state: ScanState,
}

pub struct SyntaxHighlighter {
    language: String,
    line_states: HashMap<usize, LineState>,
    keywords: HashMap<&'static str, TokenType>,
    operators: Vec<&'static str>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            language: "text".to_string(),
            line_states: HashMap::new(),
            keywords: HashMap::new(),
            operators: Vec::new(),
        }
    }

    pub fn set_language(&mut self, language: &str) {
        if self.language != language {
            self.language = language.to_string();
            self.line_states.clear();
            self.setup_language_config();
        }
    }

    fn setup_language_config(&mut self) {
        self.keywords.clear();
        self.operators.clear();

        match self.language.as_str() {
            "rust" => {
                self.setup_rust();
            }
            "python" => {
                self.setup_python();
            }
            "javascript" | "typescript" => {
                self.setup_javascript();
            }
            "bash" => {
                self.setup_bash();
            }
            "json" => {
                self.setup_json();
            }
            "sql" => {
                self.setup_sql();
            }
            "toml" => {
                self.setup_toml();
            }
            "html" => {
                self.setup_html();
            }
            "css" => {
                self.setup_css();
            }
            "markdown" => {
                self.setup_markdown();
            }
            "c" => {
                self.setup_c();
            }
            "cpp" => {
                self.setup_cpp();
            }
            "xml" => {
                self.setup_xml();
            }
            "yaml" => {
                self.setup_yaml();
            }
            "dockerfile" => {
                self.setup_dockerfile();
            }
            "makefile" => {
                self.setup_makefile();
            }
            _ => {}
        }
    }

    fn setup_rust(&mut self) {
        // Keywords
        let keywords = [
            ("let", TokenType::Keyword), ("mut", TokenType::Keyword), ("fn", TokenType::Keyword),
            ("if", TokenType::Keyword), ("else", TokenType::Keyword), ("while", TokenType::Keyword),
            ("for", TokenType::Keyword), ("loop", TokenType::Keyword), ("match", TokenType::Keyword),
            ("return", TokenType::Keyword), ("struct", TokenType::Keyword), ("enum", TokenType::Keyword),
            ("impl", TokenType::Keyword), ("trait", TokenType::Keyword), ("use", TokenType::Keyword),
            ("mod", TokenType::Keyword), ("pub", TokenType::Keyword), ("const", TokenType::Keyword),
            ("static", TokenType::Keyword), ("unsafe", TokenType::Keyword), ("async", TokenType::Keyword),
            ("await", TokenType::Keyword), ("move", TokenType::Keyword), ("ref", TokenType::Keyword),
            ("self", TokenType::Keyword), ("Self", TokenType::Type), ("super", TokenType::Keyword),
            ("crate", TokenType::Keyword), ("where", TokenType::Keyword), ("type", TokenType::Keyword),
            ("as", TokenType::Keyword), ("in", TokenType::Keyword), ("break", TokenType::Keyword),
            ("continue", TokenType::Keyword), ("true", TokenType::Constant), ("false", TokenType::Constant),
            // Types
            ("i8", TokenType::Type), ("i16", TokenType::Type), ("i32", TokenType::Type),
            ("i64", TokenType::Type), ("i128", TokenType::Type), ("isize", TokenType::Type),
            ("u8", TokenType::Type), ("u16", TokenType::Type), ("u32", TokenType::Type),
            ("u64", TokenType::Type), ("u128", TokenType::Type), ("usize", TokenType::Type),
            ("f32", TokenType::Type), ("f64", TokenType::Type), ("bool", TokenType::Type),
            ("char", TokenType::Type), ("str", TokenType::Type),
            ("String", TokenType::Type), ("Vec", TokenType::Type), ("HashMap", TokenType::Type),
            ("HashSet", TokenType::Type), ("Option", TokenType::Type), ("Result", TokenType::Type),
            ("Box", TokenType::Type), ("Rc", TokenType::Type), ("Arc", TokenType::Type),
            ("RefCell", TokenType::Type), ("Mutex", TokenType::Type), ("RwLock", TokenType::Type),
        ];
        
        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        // Order matters! Longer operators must come before shorter ones
        self.operators = vec![
            // Three character operators
            "..=", "<<=", ">>=",
            // Two character operators
            "->", "=>", "::", "<=", ">=", "==", "!=", "&&", "||",
            "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "..",
            // Single character operators
            "+", "-", "*", "/", "%", "&", "|", "^", "!", "=", "<", ">",
            ".", ",", ";", ":", "?", "@", "_", "$", "#",
            "(", ")", "[", "]", "{", "}",
        ];
    }

    fn setup_python(&mut self) {
        let keywords = [
            ("def", TokenType::Keyword), ("class", TokenType::Keyword), ("if", TokenType::Keyword),
            ("else", TokenType::Keyword), ("elif", TokenType::Keyword), ("while", TokenType::Keyword),
            ("for", TokenType::Keyword), ("in", TokenType::Keyword), ("try", TokenType::Keyword),
            ("except", TokenType::Keyword), ("finally", TokenType::Keyword), ("with", TokenType::Keyword),
            ("as", TokenType::Keyword), ("import", TokenType::Keyword), ("from", TokenType::Keyword),
            ("return", TokenType::Keyword), ("yield", TokenType::Keyword), ("pass", TokenType::Keyword),
            ("break", TokenType::Keyword), ("continue", TokenType::Keyword), ("and", TokenType::Keyword),
            ("or", TokenType::Keyword), ("not", TokenType::Keyword), ("is", TokenType::Keyword),
            ("lambda", TokenType::Keyword), ("global", TokenType::Keyword), ("nonlocal", TokenType::Keyword),
            ("True", TokenType::Constant), ("False", TokenType::Constant), ("None", TokenType::Constant),
            ("async", TokenType::Keyword), ("await", TokenType::Keyword),
            // Common built-in types
            ("int", TokenType::Type), ("float", TokenType::Type), ("str", TokenType::Type),
            ("bool", TokenType::Type), ("list", TokenType::Type), ("dict", TokenType::Type),
            ("tuple", TokenType::Type), ("set", TokenType::Type), ("bytes", TokenType::Type),
        ];

        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        self.operators = vec![
            // Three character
            "//=", "**=",
            // Two character
            "==", "!=", ">=", "<=", "+=", "-=", "*=", "/=", "%=", "**", "//", "<<", ">>",
            // Single character
            "+", "-", "*", "/", "%", "=", "<", ">", "&", "|", "^", "~", "!",
            ".", ",", ":", ";", "(", ")", "[", "]", "{", "}",
        ];
    }

    fn setup_javascript(&mut self) {
        let keywords = [
            ("function", TokenType::Keyword), ("var", TokenType::Keyword), ("let", TokenType::Keyword),
            ("const", TokenType::Keyword), ("if", TokenType::Keyword), ("else", TokenType::Keyword),
            ("while", TokenType::Keyword), ("for", TokenType::Keyword), ("do", TokenType::Keyword),
            ("switch", TokenType::Keyword), ("case", TokenType::Keyword), ("default", TokenType::Keyword),
            ("break", TokenType::Keyword), ("continue", TokenType::Keyword), ("return", TokenType::Keyword),
            ("try", TokenType::Keyword), ("catch", TokenType::Keyword), ("finally", TokenType::Keyword),
            ("throw", TokenType::Keyword), ("new", TokenType::Keyword), ("this", TokenType::Keyword),
            ("typeof", TokenType::Keyword), ("instanceof", TokenType::Keyword), ("in", TokenType::Keyword),
            ("of", TokenType::Keyword), ("true", TokenType::Constant), ("false", TokenType::Constant),
            ("null", TokenType::Constant), ("undefined", TokenType::Constant), ("class", TokenType::Keyword),
            ("extends", TokenType::Keyword), ("super", TokenType::Keyword), ("static", TokenType::Keyword),
            ("async", TokenType::Keyword), ("await", TokenType::Keyword), ("import", TokenType::Keyword),
            ("export", TokenType::Keyword), ("from", TokenType::Keyword), ("as", TokenType::Keyword),
            ("get", TokenType::Keyword), ("set", TokenType::Keyword), ("delete", TokenType::Keyword),
        ];

        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        self.operators = vec![
            // Three character
            "===", "!==", ">>>", "<<=", ">>=", "**=",
            // Two character
            "==", "!=", ">=", "<=", "&&", "||", "??", "++", "--", "+=", "-=", "*=", "/=", "%=",
            "<<", ">>", "=>", "**", "?.", "??",
            // Single character
            "+", "-", "*", "/", "%", "=", "<", ">", "!", "?", ":",
            ".", ",", ";", "(", ")", "[", "]", "{", "}", "&", "|", "^", "~",
        ];
    }

    fn setup_bash(&mut self) {
        let keywords = [
            ("if", TokenType::Keyword), ("then", TokenType::Keyword), ("else", TokenType::Keyword),
            ("elif", TokenType::Keyword), ("fi", TokenType::Keyword), ("case", TokenType::Keyword),
            ("esac", TokenType::Keyword), ("for", TokenType::Keyword), ("while", TokenType::Keyword),
            ("until", TokenType::Keyword), ("do", TokenType::Keyword), ("done", TokenType::Keyword),
            ("function", TokenType::Keyword), ("return", TokenType::Keyword), ("break", TokenType::Keyword),
            ("continue", TokenType::Keyword), ("local", TokenType::Keyword), ("export", TokenType::Keyword),
            ("readonly", TokenType::Keyword), ("declare", TokenType::Keyword), ("unset", TokenType::Keyword),
            ("source", TokenType::Keyword), ("alias", TokenType::Keyword),
        ];

        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        self.operators = vec![
            "&&", "||", "<<", ">>", "==", "!=", "<=", ">=",
            "=", "+", "-", "*", "/", "%", "!", "<", ">",
            "|", "&", ";", "(", ")", "[", "]", "{", "}",
            "$", "`", "~", ".", ",", ":", "?",
        ];
    }

    fn setup_json(&mut self) {
        // JSON has no keywords - constants are handled in scan_identifier
        self.keywords.clear();

        self.operators = vec![
            ":", ",", "{", "}", "[", "]",
        ];
    }

    fn setup_sql(&mut self) {
        let keywords = [
            // SQL keywords (uppercase)
            ("SELECT", TokenType::Keyword), ("FROM", TokenType::Keyword), ("WHERE", TokenType::Keyword),
            ("AND", TokenType::Keyword), ("OR", TokenType::Keyword), ("NOT", TokenType::Keyword),
            ("IN", TokenType::Keyword), ("LIKE", TokenType::Keyword), ("IS", TokenType::Keyword),
            ("NULL", TokenType::Constant), ("ORDER", TokenType::Keyword), ("BY", TokenType::Keyword),
            ("GROUP", TokenType::Keyword), ("HAVING", TokenType::Keyword), ("LIMIT", TokenType::Keyword),
            ("OFFSET", TokenType::Keyword), ("DISTINCT", TokenType::Keyword), ("AS", TokenType::Keyword),
            ("JOIN", TokenType::Keyword), ("LEFT", TokenType::Keyword), ("RIGHT", TokenType::Keyword),
            ("INNER", TokenType::Keyword), ("OUTER", TokenType::Keyword), ("FULL", TokenType::Keyword),
            ("CROSS", TokenType::Keyword), ("ON", TokenType::Keyword), ("UNION", TokenType::Keyword),
            ("INTERSECT", TokenType::Keyword), ("EXCEPT", TokenType::Keyword), ("ALL", TokenType::Keyword),
            ("INSERT", TokenType::Keyword), ("INTO", TokenType::Keyword), ("VALUES", TokenType::Keyword),
            ("UPDATE", TokenType::Keyword), ("SET", TokenType::Keyword), ("DELETE", TokenType::Keyword),
            ("CREATE", TokenType::Keyword), ("TABLE", TokenType::Keyword), ("ALTER", TokenType::Keyword),
            ("DROP", TokenType::Keyword), ("PRIMARY", TokenType::Keyword), ("KEY", TokenType::Keyword),
            ("FOREIGN", TokenType::Keyword), ("REFERENCES", TokenType::Keyword), ("INDEX", TokenType::Keyword),
            ("VIEW", TokenType::Keyword), ("DATABASE", TokenType::Keyword), ("SCHEMA", TokenType::Keyword),
            ("IF", TokenType::Keyword), ("EXISTS", TokenType::Keyword), ("CASCADE", TokenType::Keyword),
            ("RESTRICT", TokenType::Keyword), ("CASE", TokenType::Keyword), ("WHEN", TokenType::Keyword),
            ("THEN", TokenType::Keyword), ("ELSE", TokenType::Keyword), ("END", TokenType::Keyword),
            ("BEGIN", TokenType::Keyword), ("COMMIT", TokenType::Keyword), ("ROLLBACK", TokenType::Keyword),
            ("TRANSACTION", TokenType::Keyword), ("WITH", TokenType::Keyword), ("RECURSIVE", TokenType::Keyword),
            ("WINDOW", TokenType::Keyword), ("PARTITION", TokenType::Keyword), ("OVER", TokenType::Keyword),
            ("ROW", TokenType::Keyword), ("ROWS", TokenType::Keyword), ("BETWEEN", TokenType::Keyword),
            ("UNBOUNDED", TokenType::Keyword), ("PRECEDING", TokenType::Keyword), ("FOLLOWING", TokenType::Keyword),
            ("CURRENT", TokenType::Keyword), ("QUALIFY", TokenType::Keyword), ("FETCH", TokenType::Keyword),
            ("FIRST", TokenType::Keyword), ("NEXT", TokenType::Keyword), ("ONLY", TokenType::Keyword),
            // Lowercase versions
            ("select", TokenType::Keyword), ("from", TokenType::Keyword), ("where", TokenType::Keyword),
            ("and", TokenType::Keyword), ("or", TokenType::Keyword), ("not", TokenType::Keyword),
            ("in", TokenType::Keyword), ("like", TokenType::Keyword), ("is", TokenType::Keyword),
            ("null", TokenType::Constant), ("order", TokenType::Keyword), ("by", TokenType::Keyword),
            ("group", TokenType::Keyword), ("having", TokenType::Keyword), ("limit", TokenType::Keyword),
            ("offset", TokenType::Keyword), ("distinct", TokenType::Keyword), ("as", TokenType::Keyword),
            ("join", TokenType::Keyword), ("left", TokenType::Keyword), ("right", TokenType::Keyword),
            ("inner", TokenType::Keyword), ("outer", TokenType::Keyword), ("full", TokenType::Keyword),
            ("cross", TokenType::Keyword), ("on", TokenType::Keyword), ("union", TokenType::Keyword),
            ("intersect", TokenType::Keyword), ("except", TokenType::Keyword), ("all", TokenType::Keyword),
            ("insert", TokenType::Keyword), ("into", TokenType::Keyword), ("values", TokenType::Keyword),
            ("update", TokenType::Keyword), ("set", TokenType::Keyword), ("delete", TokenType::Keyword),
            ("create", TokenType::Keyword), ("table", TokenType::Keyword), ("alter", TokenType::Keyword),
            ("drop", TokenType::Keyword), ("primary", TokenType::Keyword), ("key", TokenType::Keyword),
            ("foreign", TokenType::Keyword), ("references", TokenType::Keyword), ("index", TokenType::Keyword),
            ("view", TokenType::Keyword), ("database", TokenType::Keyword), ("schema", TokenType::Keyword),
            ("if", TokenType::Keyword), ("exists", TokenType::Keyword), ("cascade", TokenType::Keyword),
            ("restrict", TokenType::Keyword), ("case", TokenType::Keyword), ("when", TokenType::Keyword),
            ("then", TokenType::Keyword), ("else", TokenType::Keyword), ("end", TokenType::Keyword),
            ("begin", TokenType::Keyword), ("commit", TokenType::Keyword), ("rollback", TokenType::Keyword),
            ("transaction", TokenType::Keyword), ("with", TokenType::Keyword), ("recursive", TokenType::Keyword),
            ("window", TokenType::Keyword), ("partition", TokenType::Keyword), ("over", TokenType::Keyword),
            ("row", TokenType::Keyword), ("rows", TokenType::Keyword), ("between", TokenType::Keyword),
            ("unbounded", TokenType::Keyword), ("preceding", TokenType::Keyword), ("following", TokenType::Keyword),
            ("current", TokenType::Keyword), ("qualify", TokenType::Keyword), ("fetch", TokenType::Keyword),
            ("first", TokenType::Keyword), ("next", TokenType::Keyword), ("only", TokenType::Keyword),
            // Common SQL types
            ("INTEGER", TokenType::Type), ("VARCHAR", TokenType::Type), ("TEXT", TokenType::Type),
            ("BOOLEAN", TokenType::Type), ("DATE", TokenType::Type), ("TIMESTAMP", TokenType::Type),
            ("DECIMAL", TokenType::Type), ("FLOAT", TokenType::Type), ("DOUBLE", TokenType::Type),
            ("CHAR", TokenType::Type), ("BIGINT", TokenType::Type), ("SMALLINT", TokenType::Type),
            ("NUMERIC", TokenType::Type), ("REAL", TokenType::Type), ("TIME", TokenType::Type),
            ("DATETIME", TokenType::Type), ("TIMESTAMP_NTZ", TokenType::Type), ("TIMESTAMP_LTZ", TokenType::Type),
            ("TIMESTAMP_TZ", TokenType::Type), ("VARIANT", TokenType::Type), ("OBJECT", TokenType::Type),
            ("ARRAY", TokenType::Type), ("BINARY", TokenType::Type), ("VARBINARY", TokenType::Type),
            ("STRING", TokenType::Type), ("NUMBER", TokenType::Type), ("TINYINT", TokenType::Type),
            ("MEDIUMINT", TokenType::Type), ("INT", TokenType::Type), ("SERIAL", TokenType::Type),
            ("BIGSERIAL", TokenType::Type), ("MONEY", TokenType::Type), ("INTERVAL", TokenType::Type),
            ("BLOB", TokenType::Type), ("CLOB", TokenType::Type), ("UUID", TokenType::Type),
            ("JSON", TokenType::Type), ("JSONB", TokenType::Type), ("XML", TokenType::Type),
            ("GEOGRAPHY", TokenType::Type), ("GEOMETRY", TokenType::Type), ("POINT", TokenType::Type),
            ("integer", TokenType::Type), ("varchar", TokenType::Type), ("text", TokenType::Type),
            ("boolean", TokenType::Type), ("date", TokenType::Type), ("timestamp", TokenType::Type),
            ("decimal", TokenType::Type), ("float", TokenType::Type), ("double", TokenType::Type),
            ("char", TokenType::Type), ("bigint", TokenType::Type), ("smallint", TokenType::Type),
            ("numeric", TokenType::Type), ("real", TokenType::Type), ("time", TokenType::Type),
            ("datetime", TokenType::Type), ("timestamp_ntz", TokenType::Type), ("timestamp_ltz", TokenType::Type),
            ("timestamp_tz", TokenType::Type), ("variant", TokenType::Type), ("object", TokenType::Type),
            ("array", TokenType::Type), ("binary", TokenType::Type), ("varbinary", TokenType::Type),
            ("string", TokenType::Type), ("number", TokenType::Type), ("tinyint", TokenType::Type),
            ("mediumint", TokenType::Type), ("int", TokenType::Type), ("serial", TokenType::Type),
            ("bigserial", TokenType::Type), ("money", TokenType::Type), ("interval", TokenType::Type),
            ("blob", TokenType::Type), ("clob", TokenType::Type), ("uuid", TokenType::Type),
            ("json", TokenType::Type), ("jsonb", TokenType::Type), ("xml", TokenType::Type),
            ("geography", TokenType::Type), ("geometry", TokenType::Type), ("point", TokenType::Type),
            // Common SQL functions (will be detected as functions when followed by parentheses)
            ("COUNT", TokenType::Function), ("SUM", TokenType::Function), ("AVG", TokenType::Function),
            ("MIN", TokenType::Function), ("MAX", TokenType::Function), ("CAST", TokenType::Function),
            ("CONVERT", TokenType::Function), ("COALESCE", TokenType::Function), ("NULLIF", TokenType::Function),
            ("SUBSTRING", TokenType::Function), ("LENGTH", TokenType::Function), ("TRIM", TokenType::Function),
            ("UPPER", TokenType::Function), ("LOWER", TokenType::Function), ("REPLACE", TokenType::Function),
            ("CONCAT", TokenType::Function), ("NOW", TokenType::Function), ("CURRENT_DATE", TokenType::Function),
            ("CURRENT_TIME", TokenType::Function), ("CURRENT_TIMESTAMP", TokenType::Function),
            ("DATEADD", TokenType::Function), ("DATEDIFF", TokenType::Function), ("EXTRACT", TokenType::Function),
            ("ROW_NUMBER", TokenType::Function), ("RANK", TokenType::Function), ("DENSE_RANK", TokenType::Function),
            ("LAG", TokenType::Function), ("LEAD", TokenType::Function), ("FIRST_VALUE", TokenType::Function),
            ("LAST_VALUE", TokenType::Function), ("LISTAGG", TokenType::Function), ("STRING_AGG", TokenType::Function),
            ("count", TokenType::Function), ("sum", TokenType::Function), ("avg", TokenType::Function),
            ("min", TokenType::Function), ("max", TokenType::Function), ("cast", TokenType::Function),
            ("convert", TokenType::Function), ("coalesce", TokenType::Function), ("nullif", TokenType::Function),
            ("substring", TokenType::Function), ("length", TokenType::Function), ("trim", TokenType::Function),
            ("upper", TokenType::Function), ("lower", TokenType::Function), ("replace", TokenType::Function),
            ("concat", TokenType::Function), ("now", TokenType::Function), ("current_date", TokenType::Function),
            ("current_time", TokenType::Function), ("current_timestamp", TokenType::Function),
            ("dateadd", TokenType::Function), ("datediff", TokenType::Function), ("extract", TokenType::Function),
            ("row_number", TokenType::Function), ("rank", TokenType::Function), ("dense_rank", TokenType::Function),
            ("lag", TokenType::Function), ("lead", TokenType::Function), ("first_value", TokenType::Function),
            ("last_value", TokenType::Function), ("listagg", TokenType::Function), ("string_agg", TokenType::Function),
        ];

        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        self.operators = vec![
            ">=", "<=", "<>", "!=", "==", "||", "::",
            "=", "+", "-", "*", "/", "%", "<", ">",
            "(", ")", ",", ";", ".", ":",
        ];
    }

    fn setup_toml(&mut self) {
        // TOML constants are handled in scan_identifier
        self.keywords.clear();

        self.operators = vec![
            "=", "[", "]", "{", "}", ",", ".",
        ];
    }

    fn setup_html(&mut self) {
        // HTML doesn't have keywords - tags are identified contextually
        self.keywords.clear();
        
        self.operators = vec![
            "<", ">", "</", "/>", "=",
        ];
    }

    fn setup_css(&mut self) {
        // CSS doesn't have keywords in the traditional sense
        // Properties are identified contextually in post-processing
        self.keywords.clear();

        self.operators = vec![
            "{", "}", ":", ";", ",", ".", "#", ">", "+", "~", "*",
            "(", ")", "[", "]", "=",
        ];
    }

    fn setup_markdown(&mut self) {
        // Markdown doesn't have keywords - formatting is identified contextually
        self.keywords.clear();
        
        self.operators = vec![
            "#", "*", "_", "-", "+", ">", "`", "[", "]", "(", ")",
            "!", "=", "|",
        ];
    }

    fn setup_c(&mut self) {
        let keywords = [
            // Keywords
            ("auto", TokenType::Keyword), ("break", TokenType::Keyword), ("case", TokenType::Keyword),
            ("char", TokenType::Type), ("const", TokenType::Keyword), ("continue", TokenType::Keyword),
            ("default", TokenType::Keyword), ("do", TokenType::Keyword), ("double", TokenType::Type),
            ("else", TokenType::Keyword), ("enum", TokenType::Keyword), ("extern", TokenType::Keyword),
            ("float", TokenType::Type), ("for", TokenType::Keyword), ("goto", TokenType::Keyword),
            ("if", TokenType::Keyword), ("inline", TokenType::Keyword), ("int", TokenType::Type),
            ("long", TokenType::Type), ("register", TokenType::Keyword), ("restrict", TokenType::Keyword),
            ("return", TokenType::Keyword), ("short", TokenType::Type), ("signed", TokenType::Type),
            ("sizeof", TokenType::Keyword), ("static", TokenType::Keyword), ("struct", TokenType::Keyword),
            ("switch", TokenType::Keyword), ("typedef", TokenType::Keyword), ("union", TokenType::Keyword),
            ("unsigned", TokenType::Type), ("void", TokenType::Type), ("volatile", TokenType::Keyword),
            ("while", TokenType::Keyword), ("_Bool", TokenType::Type), ("_Complex", TokenType::Type),
            ("_Imaginary", TokenType::Type),
            // Constants
            ("NULL", TokenType::Constant), ("true", TokenType::Constant), ("false", TokenType::Constant),
            // Common types
            ("size_t", TokenType::Type), ("ssize_t", TokenType::Type), ("ptrdiff_t", TokenType::Type),
            ("uint8_t", TokenType::Type), ("uint16_t", TokenType::Type), ("uint32_t", TokenType::Type),
            ("uint64_t", TokenType::Type), ("int8_t", TokenType::Type), ("int16_t", TokenType::Type),
            ("int32_t", TokenType::Type), ("int64_t", TokenType::Type), ("bool", TokenType::Type),
        ];

        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }

        self.operators = vec![
            // Three character
            "<<=", ">>=", "...",
            // Two character
            "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||",
            "+=", "-=", "*=", "/=", "%=", "&=", "^=", "|=",
            // Single character
            "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "=", "<", ">",
            ".", ",", ";", ":", "?", "(", ")", "[", "]", "{", "}",
        ];
    }

    fn setup_cpp(&mut self) {
        // Start with C keywords
        self.setup_c();
        
        // Add C++ specific keywords
        let cpp_keywords = [
            // C++ keywords
            ("alignas", TokenType::Keyword), ("alignof", TokenType::Keyword), ("and", TokenType::Keyword),
            ("and_eq", TokenType::Keyword), ("asm", TokenType::Keyword), ("bitand", TokenType::Keyword),
            ("bitor", TokenType::Keyword), ("catch", TokenType::Keyword), ("class", TokenType::Keyword),
            ("compl", TokenType::Keyword), ("concept", TokenType::Keyword), ("const_cast", TokenType::Keyword),
            ("consteval", TokenType::Keyword), ("constexpr", TokenType::Keyword), ("constinit", TokenType::Keyword),
            ("co_await", TokenType::Keyword), ("co_return", TokenType::Keyword), ("co_yield", TokenType::Keyword),
            ("decltype", TokenType::Keyword), ("delete", TokenType::Keyword), ("dynamic_cast", TokenType::Keyword),
            ("explicit", TokenType::Keyword), ("export", TokenType::Keyword), ("friend", TokenType::Keyword),
            ("mutable", TokenType::Keyword), ("namespace", TokenType::Keyword), ("new", TokenType::Keyword),
            ("noexcept", TokenType::Keyword), ("not", TokenType::Keyword), ("not_eq", TokenType::Keyword),
            ("nullptr", TokenType::Constant), ("operator", TokenType::Keyword), ("or", TokenType::Keyword),
            ("or_eq", TokenType::Keyword), ("private", TokenType::Keyword), ("protected", TokenType::Keyword),
            ("public", TokenType::Keyword), ("reinterpret_cast", TokenType::Keyword), ("requires", TokenType::Keyword),
            ("static_assert", TokenType::Keyword), ("static_cast", TokenType::Keyword), ("template", TokenType::Keyword),
            ("this", TokenType::Keyword), ("thread_local", TokenType::Keyword), ("throw", TokenType::Keyword),
            ("try", TokenType::Keyword), ("typeid", TokenType::Keyword), ("typename", TokenType::Keyword),
            ("using", TokenType::Keyword), ("virtual", TokenType::Keyword), ("xor", TokenType::Keyword),
            ("xor_eq", TokenType::Keyword),
            // Additional types
            ("string", TokenType::Type), ("wstring", TokenType::Type), ("vector", TokenType::Type),
            ("map", TokenType::Type), ("set", TokenType::Type), ("pair", TokenType::Type),
            ("unique_ptr", TokenType::Type), ("shared_ptr", TokenType::Type), ("weak_ptr", TokenType::Type),
            ("optional", TokenType::Type), ("variant", TokenType::Type), ("any", TokenType::Type),
        ];

        for (word, token_type) in cpp_keywords {
            self.keywords.insert(word, token_type);
        }

        // C++ has some additional operators
        self.operators.push("::");
        self.operators.push(".*");
        self.operators.push("->*");
    }

    fn setup_xml(&mut self) {
        // XML doesn't have keywords - tags are identified contextually
        self.keywords.clear();
        
        // XML is similar to HTML
        self.operators = vec![
            "<", ">", "</", "/>", "=", ":", "?",
        ];
    }

    fn setup_yaml(&mut self) {
        // YAML constants are handled in scan_identifier
        self.keywords.clear();

        self.operators = vec![
            ":", "-", ">", "|", "&", "*", "!", "=", "[", "]", "{", "}",
        ];
    }
    
    fn setup_dockerfile(&mut self) {
        // Dockerfile keywords (instructions)
        let keywords = [
            ("FROM", TokenType::Keyword), ("RUN", TokenType::Keyword), ("CMD", TokenType::Keyword),
            ("LABEL", TokenType::Keyword), ("EXPOSE", TokenType::Keyword), ("ENV", TokenType::Keyword),
            ("ADD", TokenType::Keyword), ("COPY", TokenType::Keyword), ("ENTRYPOINT", TokenType::Keyword),
            ("VOLUME", TokenType::Keyword), ("USER", TokenType::Keyword), ("WORKDIR", TokenType::Keyword),
            ("ARG", TokenType::Keyword), ("ONBUILD", TokenType::Keyword), ("STOPSIGNAL", TokenType::Keyword),
            ("HEALTHCHECK", TokenType::Keyword), ("SHELL", TokenType::Keyword), ("MAINTAINER", TokenType::Keyword),
            ("AS", TokenType::Keyword),
        ];
        
        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }
        
        self.operators = vec![
            "=", "[", "]", ",", "&&", "||", ";", "|", ">", "<", ">>", "<<",
        ];
    }
    
    fn setup_makefile(&mut self) {
        // Makefile keywords
        let keywords = [
            ("include", TokenType::Keyword), ("define", TokenType::Keyword), ("endef", TokenType::Keyword),
            ("ifdef", TokenType::Keyword), ("ifndef", TokenType::Keyword), ("ifeq", TokenType::Keyword),
            ("ifneq", TokenType::Keyword), ("else", TokenType::Keyword), ("endif", TokenType::Keyword),
            ("export", TokenType::Keyword), ("override", TokenType::Keyword), ("private", TokenType::Keyword),
            ("vpath", TokenType::Keyword), (".PHONY", TokenType::Keyword), (".SUFFIXES", TokenType::Keyword),
            (".DEFAULT", TokenType::Keyword), (".PRECIOUS", TokenType::Keyword), (".INTERMEDIATE", TokenType::Keyword),
            (".SECONDARY", TokenType::Keyword), (".DELETE_ON_ERROR", TokenType::Keyword),
            (".IGNORE", TokenType::Keyword), (".SILENT", TokenType::Keyword), (".EXPORT_ALL_VARIABLES", TokenType::Keyword),
        ];
        
        for (word, token_type) in keywords {
            self.keywords.insert(word, token_type);
        }
        
        self.operators = vec![
            "=", ":=", "+=", "?=", "!=", ":", ";", "|", "@", "-", "+",
            "(", ")", "$", "{", "}", "[", "]",
        ];
    }

    pub fn update(&mut self, rope: &Rope) {
        if self.language == "text" {
            self.line_states.clear();
            return;
        }

        let mut current_state = ScanState::Normal;
        let total_lines = rope.len_lines();
        
        // Add a safety limit to prevent infinite loops
        let max_lines = total_lines.min(100000); // Reasonable limit
        
        for line_idx in 0..max_lines {
            if line_idx >= total_lines {
                break;
            }
            
            let line_text = rope.line(line_idx).to_string();
            let (tokens, end_state) = self.scan_line(&line_text, current_state);
            
            self.line_states.insert(line_idx, LineState {
                tokens,
                end_state,
            });
            
            current_state = end_state;
        }
    }
    
    /// Optimized update for a range of lines (useful for bulk operations like paste)
    pub fn update_range(&mut self, rope: &Rope, start_line: usize, end_line: usize) {
        if self.language == "text" {
            return;
        }
        
        // Get the state from before the range
        let start_state = if start_line > 0 {
            self.line_states.get(&(start_line - 1))
                .map(|state| state.end_state)
                .unwrap_or(ScanState::Normal)
        } else {
            ScanState::Normal
        };
        
        let mut current_state = start_state;
        
        // Update all lines in the range
        for line_idx in start_line..=end_line.min(rope.len_lines() - 1) {
            let line_text = rope.line(line_idx).to_string();
            let (tokens, end_state) = self.scan_line(&line_text, current_state);
            
            self.line_states.insert(line_idx, LineState {
                tokens,
                end_state,
            });
            
            current_state = end_state;
        }
        
        // Check if we need to continue updating beyond the range
        if end_line < rope.len_lines() - 1 {
            let next_line_state = self.line_states.get(&(end_line + 1))
                .map(|state| state.end_state);
            
            if next_line_state != Some(current_state) {
                // State changed, need to continue updating
                for line_idx in (end_line + 1)..rope.len_lines() {
                    let line_text = rope.line(line_idx).to_string();
                    let (tokens, new_end_state) = self.scan_line(&line_text, current_state);
                    
                    let old_state = self.line_states.get(&line_idx)
                        .map(|state| state.end_state);
                    
                    self.line_states.insert(line_idx, LineState {
                        tokens,
                        end_state: new_end_state,
                    });
                    
                    // If the state didn't change, we can stop
                    if old_state == Some(new_end_state) {
                        break;
                    }
                    
                    current_state = new_end_state;
                }
            }
        }
    }

    fn scan_line(&self, line: &str, start_state: ScanState) -> (Vec<SyntaxToken>, ScanState) {
        let mut tokens = Vec::new();
        let mut state = start_state;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut current_token_start = 0;
        
        // Safety: prevent infinite loops
        let max_iterations = chars.len() * 2; // Should never need more than this
        let mut iterations = 0;

        // Handle continuing states from previous line
        match state {
            ScanState::InString { .. } => {
                current_token_start = 0;
            }
            ScanState::InMultiLineComment { .. } => {
                current_token_start = 0;
            }
            ScanState::InRawString { .. } => {
                current_token_start = 0;
            }
            _ => {}
        }

        while i < chars.len() {
            iterations += 1;
            if iterations > max_iterations {
                // Detected potential infinite loop - silently break to prevent hanging
                break;
            }
            
            let ch = chars[i];
            
            match state {
                ScanState::Normal => {
                    // Skip whitespace
                    if ch.is_whitespace() {
                        i += 1;
                        continue;
                    }
                    
                    // Check for C/C++ preprocessor directives (#include, #define, etc.)
                    else if ch == '#' && (self.language == "c" || self.language == "cpp") &&
                            (i == 0 || (i > 0 && (chars[i-1] == '\n' || chars[i-1].is_whitespace()))) {
                        let directive_start = i;
                        let mut j = i + 1;
                        
                        // Skip whitespace after #
                        while j < chars.len() && chars[j].is_whitespace() && chars[j] != '\n' {
                            j += 1;
                        }
                        
                        // Read the directive name
                        if j < chars.len() && chars[j].is_alphabetic() {
                            while j < chars.len() && chars[j].is_alphabetic() {
                                j += 1;
                            }
                            
                            // Read to end of line
                            while j < chars.len() && chars[j] != '\n' {
                                j += 1;
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Attribute, // Preprocessor directives use attribute color
                                start: directive_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Just a # character
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for Rust attributes (#[...])
                    else if ch == '#' && i + 1 < chars.len() && chars[i + 1] == '[' && self.language == "rust" {
                        // Scan the entire attribute
                        let attr_start = i;
                        let mut j = i + 2;
                        let mut bracket_depth = 1;
                        
                        while j < chars.len() && bracket_depth > 0 {
                            if chars[j] == '[' {
                                bracket_depth += 1;
                            } else if chars[j] == ']' {
                                bracket_depth -= 1;
                            }
                            j += 1;
                        }
                        
                        if bracket_depth == 0 {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Attribute,
                                start: attr_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Incomplete attribute, treat # as punctuation
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for Rust lifetime parameters ('a, 'static, etc.) and character literals
                    else if ch == '\'' && self.language == "rust" {
                        if i + 1 < chars.len() {
                            let next_char = chars[i + 1];
                            
                            // Check for lifetime parameter (starts with letter)
                            if next_char.is_alphabetic() || next_char == '_' {
                                // Could be either a lifetime or a character literal
                                // Look ahead to determine which it is
                                let mut j = i + 1;
                                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                                    j += 1;
                                }
                                
                                // If the next character after the identifier is a quote, it's a char literal
                                // Otherwise, it's a lifetime
                                if j < chars.len() && chars[j] == '\'' {
                                    // Character literal like 'a' or 'dood' (even if invalid)
                                    state = ScanState::InString { quote: ch, escaped: false };
                                    current_token_start = i;
                                    i += 1;
                                } else {
                                    // Lifetime parameter like 'a or 'static
                                    tokens.push(SyntaxToken {
                                        token_type: TokenType::Type, // Lifetimes use type color
                                        start: i,
                                        end: j,
                                    });
                                    i = j;
                                }
                            } else {
                                // Any other character after ', treat as character literal
                                state = ScanState::InString { quote: ch, escaped: false };
                                current_token_start = i;
                                i += 1;
                            }
                        } else {
                            // Just a quote at end of line
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for Markdown bold/italic
                    else if (ch == '*' || ch == '_') && self.language == "markdown" {
                        let marker = ch;
                        let start = i;
                        
                        // Check for bold (** or __)
                        if i + 1 < chars.len() && chars[i + 1] == marker {
                            // Bold marker
                            let mut j = i + 2;
                            let mut found_closing = false;
                            
                            // Find the closing bold marker
                            while j + 1 < chars.len() {
                                if chars[j] == marker && chars[j + 1] == marker {
                                    found_closing = true;
                                    j += 2;
                                    break;
                                }
                                j += 1;
                            }
                            
                            if found_closing {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Keyword, // Bold uses keyword color
                                    start,
                                    end: j,
                                });
                                i = j;
                            } else {
                                // No closing marker, treat as normal punctuation
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Punctuation,
                                    start: i,
                                    end: i + 1,
                                });
                                i += 1;
                            }
                        } else {
                            // Single marker (italic)
                            let mut j = i + 1;
                            let mut found_closing = false;
                            
                            // Find the closing italic marker
                            while j < chars.len() && chars[j] != '\n' {
                                if chars[j] == marker && (j + 1 >= chars.len() || chars[j + 1] != marker) {
                                    found_closing = true;
                                    j += 1;
                                    break;
                                }
                                j += 1;
                            }
                            
                            if found_closing {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Type, // Italic uses type color
                                    start,
                                    end: j,
                                });
                                i = j;
                            } else {
                                // No closing marker
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Punctuation,
                                    start: i,
                                    end: i + 1,
                                });
                                i += 1;
                            }
                        }
                    }
                    // Check for Markdown code blocks and inline code
                    else if ch == '`' && self.language == "markdown" {
                        let code_start = i;
                        if i + 2 < chars.len() && chars[i + 1] == '`' && chars[i + 2] == '`' {
                            // Code block
                            let mut j = i + 3;
                            // Skip to the end of the line for the opening ```
                            while j < chars.len() && chars[j] != '\n' {
                                j += 1;
                            }
                            tokens.push(SyntaxToken {
                                token_type: TokenType::String, // Code blocks use string color
                                start: code_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Inline code - find the closing `
                            let mut j = i + 1;
                            while j < chars.len() && chars[j] != '`' && chars[j] != '\n' {
                                j += 1;
                            }
                            if j < chars.len() && chars[j] == '`' {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::String, // Inline code uses string color
                                    start: code_start,
                                    end: j + 1,
                                });
                                i = j + 1;
                            } else {
                                // Unclosed inline code
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Punctuation,
                                    start: i,
                                    end: i + 1,
                                });
                                i += 1;
                            }
                        }
                    }
                    // Check for SQL quoted identifiers (double quotes can be identifiers in SQL)
                    else if ch == '"' && self.language == "sql" {
                        // In SQL, double quotes often denote identifiers, not strings
                        // But we'll treat them as strings for now and let context determine
                        state = ScanState::InString { quote: ch, escaped: false };
                        current_token_start = i;
                        i += 1;
                    }
                    // Check for string start (including Python docstrings and TOML strings)
                    else if ch == '"' || (ch == '\'' && !matches!(self.language.as_str(), "markdown" | "toml")) {
                        // TOML uses single quotes for literal strings (no escaping)
                        // Check for Python triple-quoted strings
                        if self.language == "python" && i + 2 < chars.len() && 
                           chars[i + 1] == ch && chars[i + 2] == ch {
                            // Triple-quoted string (docstring)
                            current_token_start = i;
                            let quote_char = ch;
                            i += 3; // Skip the triple quotes
                            
                            // Find the closing triple quotes
                            let mut found_end = false;
                            while i + 2 < chars.len() {
                                if chars[i] == quote_char && chars[i + 1] == quote_char && chars[i + 2] == quote_char {
                                    i += 3;
                                    found_end = true;
                                    break;
                                }
                                i += 1;
                            }
                            
                            if !found_end {
                                // Unclosed triple-quoted string, highlight to end of line
                                i = chars.len();
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::String, // Docstrings are strings
                                start: current_token_start,
                                end: i,
                            });
                        }
                        // Check for TOML multi-line strings
                        else if self.language == "toml" && i + 2 < chars.len() && 
                                chars[i + 1] == ch && chars[i + 2] == ch {
                            // TOML multi-line string
                            current_token_start = i;
                            let quote_char = ch;
                            i += 3; // Skip the triple quotes
                            
                            // Find the closing triple quotes
                            let mut found_end = false;
                            while i + 2 < chars.len() {
                                if chars[i] == quote_char && chars[i + 1] == quote_char && chars[i + 2] == quote_char {
                                    i += 3;
                                    found_end = true;
                                    break;
                                }
                                i += 1;
                            }
                            
                            if !found_end {
                                // Unclosed multi-line string
                                i = chars.len();
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::String,
                                start: current_token_start,
                                end: i,
                            });
                        } else {
                            // Regular string
                            state = ScanState::InString { quote: ch, escaped: false };
                            current_token_start = i;
                            i += 1;
                        }
                    }
                    // TOML literal strings with single quotes
                    else if ch == '\'' && self.language == "toml" {
                        // Check for triple-quoted literal string
                        if i + 2 < chars.len() && chars[i + 1] == '\'' && chars[i + 2] == '\'' {
                            // TOML multi-line literal string
                            current_token_start = i;
                            i += 3; // Skip the triple quotes
                            
                            // Find the closing triple quotes (no escape processing)
                            let mut found_end = false;
                            while i + 2 < chars.len() {
                                if chars[i] == '\'' && chars[i + 1] == '\'' && chars[i + 2] == '\'' {
                                    i += 3;
                                    found_end = true;
                                    break;
                                }
                                i += 1;
                            }
                            
                            if !found_end {
                                i = chars.len();
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::String,
                                start: current_token_start,
                                end: i,
                            });
                        } else {
                            // Regular literal string (no escaping)
                            current_token_start = i;
                            i += 1;
                            
                            // Find the closing quote
                            while i < chars.len() && chars[i] != '\'' {
                                i += 1;
                            }
                            
                            if i < chars.len() && chars[i] == '\'' {
                                i += 1;
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::String,
                                start: current_token_start,
                                end: i,
                            });
                        }
                    }
                    // Check for Markdown headers (# Header) - must be at line start
                    else if ch == '#' && self.language == "markdown" && 
                            (i == 0 || (i > 0 && chars[i-1] == '\n')) {
                        // Count the number of # characters
                        let header_start = i;
                        let mut j = i;
                        while j < chars.len() && chars[j] == '#' && (j - i) < 6 {
                            j += 1;
                        }
                        // Check if followed by a space (valid header)
                        if j < chars.len() && chars[j] == ' ' {
                            // Find the end of the line
                            while j < chars.len() && chars[j] != '\n' {
                                j += 1;
                            }
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Keyword, // Headers use keyword color
                                start: header_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Not a valid header, treat as punctuation
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for Markdown unordered list markers (*, -, +) at line start or after whitespace
                    else if (ch == '*' || ch == '-' || ch == '+') && self.language == "markdown" &&
                           (i == 0 || (i > 0 && chars[i-1].is_whitespace())) &&
                           i + 1 < chars.len() && chars[i + 1] == ' ' {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Operator, // List markers use operator color
                            start: i,
                            end: i + 1,
                        });
                        i += 1;
                    }
                    // Check for Markdown ordered list markers (1. 2. etc.) at line start
                    else if ch.is_ascii_digit() && self.language == "markdown" &&
                            (i == 0 || (i > 0 && chars[i-1] == '\n')) {
                        let list_start = i;
                        let mut j = i;
                        while j < chars.len() && chars[j].is_ascii_digit() {
                            j += 1;
                        }
                        if j < chars.len() && chars[j] == '.' && 
                           j + 1 < chars.len() && chars[j + 1] == ' ' {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator, // List markers use operator color
                                start: list_start,
                                end: j + 1,
                            });
                            i = j + 1;
                        } else {
                            // Not a list marker, process as number
                            let (token, new_i) = self.scan_number(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for Markdown links [text](url)
                    else if ch == '[' && self.language == "markdown" {
                        let link_start = i;
                        let mut j = i + 1;
                        let mut found_closing = false;
                        
                        // Find the closing ]
                        while j < chars.len() && chars[j] != '\n' {
                            if chars[j] == ']' {
                                found_closing = true;
                                j += 1;
                                break;
                            }
                            j += 1;
                        }
                        
                        if found_closing && j < chars.len() && chars[j] == '(' {
                            // This is a link
                            j += 1;
                            // Find the closing )
                            while j < chars.len() && chars[j] != '\n' && chars[j] != ')' {
                                j += 1;
                            }
                            if j < chars.len() && chars[j] == ')' {
                                j += 1;
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::String, // Links use string color
                                    start: link_start,
                                    end: j,
                                });
                                i = j;
                            } else {
                                // Unclosed link
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Punctuation,
                                    start: i,
                                    end: i + 1,
                                });
                                i += 1;
                            }
                        } else {
                            // Just a bracket
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for SQL typecasting (::type)
                    else if ch == ':' && i + 1 < chars.len() && chars[i + 1] == ':' && self.language == "sql" {
                        // Look ahead to see if this is followed by a type name
                        let mut j = i + 2;
                        
                        // Skip optional whitespace
                        while j < chars.len() && chars[j].is_whitespace() && chars[j] != '\n' {
                            j += 1;
                        }
                        
                        // Check if followed by a valid identifier (type name)
                        if j < chars.len() && (chars[j].is_alphabetic() || chars[j] == '_') {
                            // Scan the type name
                            let type_start = j;
                            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                                j += 1;
                            }
                            
                            // Create tokens for :: and the type
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator,
                                start: i,
                                end: i + 2,
                            });
                            
                            let type_name: String = chars[type_start..j].iter().collect();
                            // Check if it's a known SQL type
                            let token_type = if self.keywords.contains_key(type_name.to_uppercase().as_str()) ||
                                               self.keywords.contains_key(type_name.to_lowercase().as_str()) {
                                TokenType::Type
                            } else {
                                // Even if not in our keyword list, treat it as a type in this context
                                TokenType::Type
                            };
                            
                            tokens.push(SyntaxToken {
                                token_type,
                                start: type_start,
                                end: j,
                            });
                            
                            i = j;
                        } else {
                            // Just :: operator
                            let (token, new_i) = self.scan_operator(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for comments
                    else if ch == '/' && i + 1 < chars.len() {
                        if chars[i + 1] == '/' && (self.language == "rust" || self.language == "javascript" || 
                                                   self.language == "typescript" || self.language == "c" || 
                                                   self.language == "cpp" || self.language == "css") {
                            state = ScanState::InSingleLineComment;
                            current_token_start = i;
                            i += 2;
                        } else if chars[i + 1] == '*' && (self.language == "rust" || self.language == "javascript" || 
                                                   self.language == "typescript" || self.language == "c" || 
                                                   self.language == "cpp" || self.language == "css" || self.language == "sql") {
                            state = ScanState::InMultiLineComment { depth: 1 };
                            current_token_start = i;
                            i += 2;
                        } else {
                            // Just a slash operator
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for HTML/XML tags
                    else if ch == '<' && (self.language == "html" || self.language == "xml") {
                        // Check if this is a comment
                        if i + 3 < chars.len() && chars[i + 1] == '!' && 
                           chars[i + 2] == '-' && chars[i + 3] == '-' {
                            // HTML/XML comment
                            let comment_start = i;
                            i += 4;
                            
                            // Find the closing -->
                            let mut found_end = false;
                            while i + 2 < chars.len() {
                                if chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>' {
                                    i += 3;
                                    found_end = true;
                                    break;
                                }
                                i += 1;
                            }
                            
                            if !found_end {
                                i = chars.len();
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Comment,
                                start: comment_start,
                                end: i,
                            });
                        } else {
                            // Start of a tag
                            let tag_start = i;
                            let mut j = i + 1;
                            
                            // Skip '/' for closing tags
                            if j < chars.len() && chars[j] == '/' {
                                j += 1;
                            }
                            
                            // Read the tag name
                            let tag_name_start = j;
                            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_') {
                                j += 1;
                            }
                            let tag_name_end = j;
                            
                            // Emit '<' or '</' as operator
                            if tag_name_start > i + 1 {
                                // Closing tag
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Operator,
                                    start: i,
                                    end: tag_name_start,
                                });
                            } else {
                                // Opening tag
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Operator,
                                    start: i,
                                    end: i + 1,
                                });
                            }
                            
                            // Emit tag name
                            if tag_name_end > tag_name_start {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Tag,
                                    start: tag_name_start,
                                    end: tag_name_end,
                                });
                            }
                            
                            // Continue scanning from after tag name
                            i = tag_name_end;
                            
                            // Process attributes until we find '>' or '/>'
                            while i < chars.len() && chars[i] != '>' {
                                if chars[i].is_whitespace() {
                                    i += 1;
                                } else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '>' {
                                    // Self-closing tag
                                    tokens.push(SyntaxToken {
                                        token_type: TokenType::Operator,
                                        start: i,
                                        end: i + 2,
                                    });
                                    i += 2;
                                    break;
                                } else if chars[i].is_alphabetic() || chars[i] == '_' {
                                    // Attribute name
                                    let attr_start = i;
                                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '-' || chars[i] == '_' || chars[i] == ':') {
                                        i += 1;
                                    }
                                    tokens.push(SyntaxToken {
                                        token_type: TokenType::Attribute,
                                        start: attr_start,
                                        end: i,
                                    });
                                    
                                    // Skip whitespace
                                    while i < chars.len() && chars[i].is_whitespace() {
                                        i += 1;
                                    }
                                    
                                    // Check for '='
                                    if i < chars.len() && chars[i] == '=' {
                                        tokens.push(SyntaxToken {
                                            token_type: TokenType::Operator,
                                            start: i,
                                            end: i + 1,
                                        });
                                        i += 1;
                                        
                                        // Skip whitespace
                                        while i < chars.len() && chars[i].is_whitespace() {
                                            i += 1;
                                        }
                                        
                                        // Attribute value (string)
                                        if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                                            let quote = chars[i];
                                            let value_start = i;
                                            i += 1;
                                            while i < chars.len() && chars[i] != quote {
                                                if chars[i] == '\\' && i + 1 < chars.len() {
                                                    i += 2;
                                                } else {
                                                    i += 1;
                                                }
                                            }
                                            if i < chars.len() {
                                                i += 1;
                                            }
                                            tokens.push(SyntaxToken {
                                                token_type: TokenType::String,
                                                start: value_start,
                                                end: i,
                                            });
                                        }
                                    }
                                } else {
                                    // Unknown character, skip it
                                    i += 1;
                                }
                            }
                            
                            // Emit closing '>'
                            if i < chars.len() && chars[i] == '>' {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Operator,
                                    start: i,
                                    end: i + 1,
                                });
                                i += 1;
                            }
                        }
                    }
                    else if ch == '#' && (self.language == "python" || self.language == "toml" || 
                                          self.language == "bash" || self.language == "yaml") {
                        state = ScanState::InSingleLineComment;
                        current_token_start = i;
                        i += 1;
                    }
                    else if ch == '-' && i + 1 < chars.len() && chars[i + 1] == '-' && self.language == "sql" {
                        state = ScanState::InSingleLineComment;
                        current_token_start = i;
                        i += 2;
                    }
                    // Check for raw strings (Rust)
                    else if ch == 'r' && i + 1 < chars.len() && self.language == "rust" {
                        let mut delimiter_len = 0;
                        let mut j = i + 1;
                        while j < chars.len() && chars[j] == '#' {
                            delimiter_len += 1;
                            j += 1;
                        }
                        if j < chars.len() && chars[j] == '"' {
                            state = ScanState::InRawString { delimiter_len };
                            current_token_start = i;
                            i = j + 1;
                        } else {
                            // Not a raw string, process as identifier
                            let (token, new_i) = self.scan_identifier(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for Python decorators (@decorator)
                    else if ch == '@' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() && self.language == "python" {
                        let decorator_start = i;
                        let mut j = i + 1;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                            j += 1;
                        }
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Attribute, // Decorators use attribute color
                            start: decorator_start,
                            end: j,
                        });
                        i = j;
                    }
                    // Check for CSS selectors (#id, .class)
                    else if (ch == '#' || ch == '.') && i + 1 < chars.len() && 
                            (chars[i + 1].is_alphabetic() || chars[i + 1] == '-' || chars[i + 1] == '_') && 
                            self.language == "css" {
                        let selector_start = i;
                        let mut j = i + 1;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '-' || chars[j] == '_') {
                            j += 1;
                        }
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Type, // CSS selectors use type color
                            start: selector_start,
                            end: j,
                        });
                        i = j;
                    }
                    else if ch.is_ascii_digit() {
                        let (token, new_i) = self.scan_number(&chars, i);
                        tokens.push(token);
                        i = new_i;
                    }
                    // Check for YAML anchors (&anchor)
                    else if ch == '&' && self.language == "yaml" && 
                            i + 1 < chars.len() && (chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
                        let anchor_start = i;
                        let mut j = i + 1;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-') {
                            j += 1;
                        }
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Type, // Anchors use type color
                            start: anchor_start,
                            end: j,
                        });
                        i = j;
                    }
                    // Check for YAML aliases (*alias)
                    else if ch == '*' && self.language == "yaml" && 
                            i + 1 < chars.len() && (chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
                        let alias_start = i;
                        let mut j = i + 1;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-') {
                            j += 1;
                        }
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Type, // Aliases use type color
                            start: alias_start,
                            end: j,
                        });
                        i = j;
                    }
                    // Check for YAML tags (!tag)
                    else if ch == '!' && self.language == "yaml" {
                        let tag_start = i;
                        let mut j = i + 1;
                        // Skip the second ! for !!tags
                        if j < chars.len() && chars[j] == '!' {
                            j += 1;
                        }
                        // Read the tag name
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-' || chars[j] == ':') {
                            j += 1;
                        }
                        if j > tag_start + 1 {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Attribute, // Tags use attribute color
                                start: tag_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Just a single ! punctuation
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for YAML multi-line strings (| and >)
                    else if (ch == '|' || ch == '>') && self.language == "yaml" {
                        // Check if this is at the start of a value (after colon and whitespace)
                        let mut is_multiline_indicator = false;
                        let mut j = i - 1;
                        
                        // Look back for a colon
                        while j > 0 && chars[j].is_whitespace() && chars[j] != '\n' {
                            j -= 1;
                        }
                        
                        if j >= 0 && chars[j] == ':' {
                            is_multiline_indicator = true;
                        }
                        
                        if is_multiline_indicator {
                            // This is a multi-line string indicator
                            let indicator_start = i;
                            let mut k = i + 1;
                            
                            // Skip optional chomping indicators (+, -)
                            if k < chars.len() && (chars[k] == '+' || chars[k] == '-') {
                                k += 1;
                            }
                            
                            // Skip optional indentation indicator (digit)
                            if k < chars.len() && chars[k].is_ascii_digit() {
                                k += 1;
                            }
                            
                            // Alternatively, indentation indicator can come before chomping
                            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                                k = i + 2;
                                if k < chars.len() && (chars[k] == '+' || chars[k] == '-') {
                                    k += 1;
                                }
                            }
                            
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator, // Multi-line indicators use operator color
                                start: indicator_start,
                                end: k,
                            });
                            
                            // The actual multi-line string content will be on subsequent lines
                            // and will be handled as normal text
                            i = k;
                        } else {
                            // Regular pipe operator
                            let (token, new_i) = self.scan_operator(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for YAML keys (text before colon)
                    else if self.language == "yaml" && ch.is_alphabetic() {
                        // Look ahead to see if this is a key
                        let key_start = i;
                        let mut j = i;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-' || chars[j] == '.') {
                            j += 1;
                        }
                        
                        // Skip whitespace
                        let mut k = j;
                        while k < chars.len() && chars[k].is_whitespace() && chars[k] != '\n' {
                            k += 1;
                        }
                        
                        // Check if followed by colon
                        if k < chars.len() && chars[k] == ':' &&
                           (k + 1 >= chars.len() || chars[k + 1].is_whitespace() || chars[k + 1] == '\n') {
                            // This is a key
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Property, // YAML keys use property color
                                start: key_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Regular identifier
                            let (token, new_i) = self.scan_identifier(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for TOML keys (similar to YAML)
                    else if self.language == "toml" && ch.is_alphabetic() {
                        // Look ahead to see if this is a key
                        let key_start = i;
                        let mut j = i;
                        while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '-' || chars[j] == '.') {
                            j += 1;
                        }
                        
                        // Skip whitespace
                        let mut k = j;
                        while k < chars.len() && chars[k].is_whitespace() && chars[k] != '\n' {
                            k += 1;
                        }
                        
                        // Check if followed by =
                        if k < chars.len() && chars[k] == '=' {
                            // This is a key
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Property,
                                start: key_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Regular identifier
                            let (token, new_i) = self.scan_identifier(&chars, i);
                            tokens.push(token);
                            i = new_i;
                        }
                    }
                    // Check for identifiers and keywords
                    else if ch.is_alphabetic() || ch == '_' {
                        // Special handling for HTML/XML - only highlight within tags
                        if (self.language == "html" || self.language == "xml") {
                            // Check if we're inside a tag by looking backwards for '<'
                            let mut inside_tag = false;
                            let mut j = i;
                            while j > 0 {
                                j -= 1;
                                if chars[j] == '>' {
                                    break; // We're not in a tag
                                } else if chars[j] == '<' {
                                    inside_tag = true;
                                    break;
                                }
                            }
                            
                            if !inside_tag {
                                // Just regular text content, don't highlight
                                let text_start = i;
                                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '<' && chars[i] != '>' {
                                    i += 1;
                                }
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::Normal,
                                    start: text_start,
                                    end: i,
                                });
                                continue;
                            }
                        }
                        
                        let (token, new_i) = self.scan_identifier(&chars, i);
                        tokens.push(token);
                        i = new_i;
                    }
                    // Check for TOML table headers [table] and [[array]]
                    else if ch == '[' && self.language == "toml" && 
                            (i == 0 || (i > 0 && chars[i-1] == '\n')) {
                        let bracket_start = i;
                        let mut j = i + 1;
                        let mut double_bracket = false;
                        
                        // Check for [[
                        if j < chars.len() && chars[j] == '[' {
                            double_bracket = true;
                            j += 1;
                        }
                        
                        // Find the closing bracket(s)
                        let content_start = j;
                        while j < chars.len() && chars[j] != ']' && chars[j] != '\n' {
                            j += 1;
                        }
                        
                        if j < chars.len() && chars[j] == ']' {
                            let content_end = j;
                            j += 1;
                            
                            // For double brackets, check for the second ]
                            if double_bracket && j < chars.len() && chars[j] == ']' {
                                j += 1;
                            }
                            
                            // Emit the entire table header as a keyword
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Keyword,
                                start: bracket_start,
                                end: j,
                            });
                            i = j;
                        } else {
                            // Just a regular bracket
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Punctuation,
                                start: i,
                                end: i + 1,
                            });
                            i += 1;
                        }
                    }
                    // Check for Makefile variables $(VAR) and automatic variables
                    else if ch == '$' && self.language == "makefile" && i + 1 < chars.len() {
                        let var_start = i;
                        i += 1;
                        
                        if chars[i] == '(' || chars[i] == '{' {
                            let close_char = if chars[i] == '(' { ')' } else { '}' };
                            i += 1;
                            while i < chars.len() && chars[i] != close_char {
                                i += 1;
                            }
                            if i < chars.len() && chars[i] == close_char {
                                i += 1;
                            }
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Variable,
                                start: var_start,
                                end: i,
                            });
                        } else if chars[i] == '@' || chars[i] == '<' || chars[i] == '^' || 
                                  chars[i] == '+' || chars[i] == '*' || chars[i] == '?' ||
                                  chars[i] == '%' || chars[i].is_ascii_digit() {
                            // Automatic variables like $@, $<, $^, etc.
                            i += 1;
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Variable,
                                start: var_start,
                                end: i,
                            });
                        } else {
                            // Just a $ character
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator,
                                start: var_start,
                                end: i,
                            });
                        }
                    }
                    // Check for Bash/Shell variables ($VAR, ${VAR})
                    else if ch == '$' && self.language == "bash" && i + 1 < chars.len() {
                        let var_start = i;
                        i += 1;
                        
                        if chars[i] == '{' {
                            // ${VAR} syntax
                            i += 1;
                            while i < chars.len() && chars[i] != '}' && !chars[i].is_whitespace() {
                                i += 1;
                            }
                            if i < chars.len() && chars[i] == '}' {
                                i += 1;
                            }
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Variable,
                                start: var_start,
                                end: i,
                            });
                        } else if chars[i].is_alphabetic() || chars[i] == '_' {
                            // $VAR syntax
                            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                                i += 1;
                            }
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Variable,
                                start: var_start,
                                end: i,
                            });
                        } else if chars[i].is_ascii_digit() || chars[i] == '@' || chars[i] == '*' || 
                                  chars[i] == '#' || chars[i] == '?' || chars[i] == '-' || chars[i] == '$' {
                            // Special variables like $1, $@, $*, $#, $?, $-, $$
                            i += 1;
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Variable,
                                start: var_start,
                                end: i,
                            });
                        } else {
                            // Just a $ character
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Operator,
                                start: var_start,
                                end: i,
                            });
                        }
                    }
                    // Check for operators and punctuation
                    else {
                        let (token, new_i) = self.scan_operator(&chars, i);
                        tokens.push(token);
                        i = new_i;
                    }
                }
                
                ScanState::InString { quote, escaped } => {
                    if escaped {
                        state = ScanState::InString { quote, escaped: false };
                        i += 1;
                    } else if ch == '\\' && !(self.language == "toml" && quote == '\'') {
                        // Escaping not supported in TOML literal strings (single quotes)
                        state = ScanState::InString { quote, escaped: true };
                        i += 1;
                    } else if ch == quote {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::String,
                            start: current_token_start,
                            end: i + 1,
                        });
                        state = ScanState::Normal;
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                
                ScanState::InSingleLineComment => {
                    // Single line comments continue until end of line
                    i += 1;
                }
                
                ScanState::InMultiLineComment { depth } => {
                    if ch == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                        if depth == 1 {
                            tokens.push(SyntaxToken {
                                token_type: TokenType::Comment,
                                start: current_token_start,
                                end: i + 2,
                            });
                            state = ScanState::Normal;
                            i += 2;
                        } else {
                            state = ScanState::InMultiLineComment { depth: depth - 1 };
                            i += 2;
                        }
                    } else if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' && self.language == "rust" {
                        // Nested comment (only Rust supports this)
                        state = ScanState::InMultiLineComment { depth: depth + 1 };
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                
                ScanState::InRawString { delimiter_len } => {
                    if ch == '"' {
                        // Check if we have enough characters left for the closing delimiter
                        if i + delimiter_len < chars.len() {
                            // Check if followed by the right number of #
                            let mut matched = true;
                            for j in 0..delimiter_len {
                                if chars[i + 1 + j] != '#' {
                                    matched = false;
                                    break;
                                }
                            }
                            if matched {
                                tokens.push(SyntaxToken {
                                    token_type: TokenType::String,
                                    start: current_token_start,
                                    end: i + 1 + delimiter_len,
                                });
                                state = ScanState::Normal;
                                i += 1 + delimiter_len;
                            } else {
                                i += 1;
                            }
                        } else {
                            // Not enough characters for closing delimiter
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
        
        // Handle end of line
        match state {
            ScanState::Normal => {
                // Nothing to do
            }
            ScanState::InSingleLineComment => {
                // Add the comment token
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    start: current_token_start,
                    end: line.len(),
                });
                // Reset to normal for next line
                state = ScanState::Normal;
            }
            ScanState::InString { .. } | ScanState::InMultiLineComment { .. } | ScanState::InRawString { .. } => {
                // These states continue to the next line
                // Add a token for the current line portion
                let token_type = match state {
                    ScanState::InString { .. } | ScanState::InRawString { .. } => TokenType::String,
                    ScanState::InMultiLineComment { .. } => TokenType::Comment,
                    _ => TokenType::Normal,
                };
                tokens.push(SyntaxToken {
                    token_type,
                    start: current_token_start,
                    end: line.len(),
                });
            }
        }
        
        // Post-process tokens to identify functions, attributes, etc.
        self.post_process_tokens(&mut tokens, &chars);
        
        (tokens, state)
    }

    fn scan_number(&self, chars: &[char], start: usize) -> (SyntaxToken, usize) {
        let mut i = start;
        let mut has_dot = false;
        let mut has_exp = false;
        
        // Handle hex, octal, binary literals
        if i + 2 < chars.len() && chars[i] == '0' {
            if chars[i + 1] == 'x' || chars[i + 1] == 'X' {
                // Hexadecimal
                i += 2;
                while i < chars.len() && chars[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else if chars[i + 1] == 'o' || chars[i + 1] == 'O' {
                // Octal
                i += 2;
                while i < chars.len() && chars[i] >= '0' && chars[i] <= '7' {
                    i += 1;
                }
            } else if chars[i + 1] == 'b' || chars[i + 1] == 'B' {
                // Binary
                i += 2;
                while i < chars.len() && (chars[i] == '0' || chars[i] == '1') {
                    i += 1;
                }
            } else {
                // Regular number starting with 0
                i += 1;
            }
        } else {
            // Regular decimal number
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        
        // Handle decimal point
        if i < chars.len() && chars[i] == '.' && !has_dot {
            // Check if next char is a digit
            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                has_dot = true;
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
        }
        
        // Handle scientific notation
        if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') && !has_exp {
            let exp_start = i;
            i += 1;
            if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                i += 1;
            }
            if i < chars.len() && chars[i].is_ascii_digit() {
                has_exp = true;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            } else {
                // Not valid scientific notation, backtrack
                i = exp_start;
            }
        }
        
        // Handle type suffixes (language-specific)
        if self.language == "rust" {
            // Rust numeric suffixes: i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
            let suffix_start = i;
            if i < chars.len() && (chars[i] == 'i' || chars[i] == 'u' || chars[i] == 'f') {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                // Handle 'size' suffix
                if i + 3 < chars.len() && chars[i..i+4] == ['s', 'i', 'z', 'e'] {
                    i += 4;
                }
            }
            // Validate the suffix
            let suffix: String = chars[suffix_start..i].iter().collect();
            if !matches!(suffix.as_str(), "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | 
                                         "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | 
                                         "f32" | "f64") {
                // Invalid suffix, backtrack
                i = suffix_start;
            }
        } else if self.language == "cpp" || self.language == "c" {
            // C/C++ numeric suffixes: L, LL, U, UL, ULL, f, F, l
            while i < chars.len() && (chars[i] == 'L' || chars[i] == 'l' || 
                                     chars[i] == 'U' || chars[i] == 'u' || 
                                     chars[i] == 'F' || chars[i] == 'f') {
                i += 1;
            }
        }
        
        (SyntaxToken {
            token_type: TokenType::Number,
            start,
            end: i,
        }, i)
    }

    fn scan_identifier(&self, chars: &[char], start: usize) -> (SyntaxToken, usize) {
        let mut i = start;
        
        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        
        let word: String = chars[start..i].iter().collect();
        
        // Language-specific handling
        let token_type = match self.language.as_str() {
            // Markup and data languages - no keyword highlighting
            "markdown" | "html" | "xml" | "css" => TokenType::Identifier,
            
            // JSON - only specific constants
            "json" => {
                if matches!(word.as_str(), "true" | "false" | "null") {
                    TokenType::Constant
                } else {
                    TokenType::Identifier
                }
            }
            
            // YAML - more constants but still limited
            "yaml" => {
                if matches!(word.as_str(), "true" | "false" | "null" | "yes" | "no" | "on" | "off") {
                    TokenType::Constant
                } else {
                    TokenType::Identifier
                }
            }
            
            // TOML - similar to JSON
            "toml" => {
                if matches!(word.as_str(), "true" | "false") {
                    TokenType::Constant
                } else {
                    TokenType::Identifier
                }
            }
            
            // SQL - case insensitive keywords
            "sql" => {
                if let Some(token_type) = self.keywords.get(word.as_str()) {
                    token_type.clone()
                } else if let Some(token_type) = self.keywords.get(word.to_uppercase().as_str()) {
                    token_type.clone()
                } else if let Some(token_type) = self.keywords.get(word.to_lowercase().as_str()) {
                    token_type.clone()
                } else {
                    TokenType::Identifier
                }
            }
            
            // Dockerfile - keywords only at line start
            "dockerfile" => {
                if let Some(token_type) = self.keywords.get(word.as_str()) {
                    // Check if this is at the start of a line (allowing for whitespace)
                    let mut at_line_start = start == 0;
                    if !at_line_start && start > 0 {
                        let mut j = start - 1;
                        at_line_start = true;
                        while j > 0 {
                            if chars[j] == '\n' {
                                break;
                            } else if !chars[j].is_whitespace() {
                                at_line_start = false;
                                break;
                            }
                            j -= 1;
                        }
                    }
                    
                    if at_line_start {
                        token_type.clone()
                    } else {
                        TokenType::Identifier
                    }
                } else {
                    TokenType::Identifier
                }
            }
            
            // All other languages - normal keyword lookup
            _ => {
                if let Some(token_type) = self.keywords.get(word.as_str()) {
                    token_type.clone()
                } else {
                    TokenType::Identifier
                }
            }
        };
        
        (SyntaxToken {
            token_type,
            start,
            end: i,
        }, i)
    }

    fn scan_operator(&self, chars: &[char], start: usize) -> (SyntaxToken, usize) {
        // Try to match multi-character operators first
        for op in &self.operators {
            if start + op.len() <= chars.len() {
                let slice: String = chars[start..start + op.len()].iter().collect();
                if slice == *op {
                    return (SyntaxToken {
                        token_type: TokenType::Operator,
                        start,
                        end: start + op.len(),
                    }, start + op.len());
                }
            }
        }
        
        // Single character punctuation
        (SyntaxToken {
            token_type: TokenType::Punctuation,
            start,
            end: start + 1,
        }, start + 1)
    }

    fn post_process_tokens(&self, tokens: &mut Vec<SyntaxToken>, chars: &[char]) {
        // Skip most post-processing for markup and data languages
        if matches!(self.language.as_str(), "markdown" | "html" | "xml" | "json" | "yaml" | "toml" | "css") {
            // For CSS, do minimal processing for selectors
            if self.language == "css" {
                self.post_process_css_tokens(tokens, chars);
            }
            // For JSON, ensure property names are styled correctly
            else if self.language == "json" {
                self.post_process_json_tokens(tokens, chars);
            }
            return;
        }
        
        // Process tokens to identify functions, types, etc. for programming languages
        for i in 0..tokens.len() {
            if tokens[i].token_type == TokenType::Identifier {
                let token_text: String = chars[tokens[i].start..tokens[i].end].iter().collect();
                
                // Check if followed by '(' for function calls - not in Markdown
                if i + 1 < tokens.len() && 
                   (tokens[i + 1].token_type == TokenType::Punctuation || tokens[i + 1].token_type == TokenType::Operator) &&
                   tokens[i + 1].start < chars.len() &&
                   chars[tokens[i + 1].start] == '(' &&
                   self.language != "markdown" {
                    tokens[i].token_type = TokenType::Function;
                }
                // Also check for method calls (preceded by '.') - not in Markdown
                else if i > 1 && 
                        (tokens[i - 1].token_type == TokenType::Punctuation || tokens[i - 1].token_type == TokenType::Operator) &&
                        tokens[i - 1].start < chars.len() &&
                        chars[tokens[i - 1].start] == '.' &&
                        i + 1 < tokens.len() && 
                        (tokens[i + 1].token_type == TokenType::Punctuation || tokens[i + 1].token_type == TokenType::Operator) &&
                        tokens[i + 1].start < chars.len() &&
                        chars[tokens[i + 1].start] == '(' &&
                        self.language != "markdown" {
                    tokens[i].token_type = TokenType::Function;
                }
                // Check if it's a type (starts with uppercase) - only for programming languages
                else if token_text.chars().next().unwrap_or('a').is_uppercase() && 
                        !matches!(self.language.as_str(), "markdown" | "text" | "json" | "toml" | "yaml" | "html" | "xml") {
                    // In Rust, also check if it might be a constant (all caps with underscores)
                    if self.language == "rust" && token_text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) &&
                       token_text.chars().any(|c| c == '_') {
                        tokens[i].token_type = TokenType::Constant;
                    } else {
                        tokens[i].token_type = TokenType::Type;
                    }
                }
                // Check for constants in programming languages
                else if token_text.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric()) &&
                        token_text.len() > 1 &&
                        token_text.chars().filter(|&c| c.is_alphabetic()).count() > 0 &&
                        !matches!(self.language.as_str(), "markdown" | "text" | "json" | "toml" | "yaml" | "html" | "xml") {
                    tokens[i].token_type = TokenType::Constant;
                }
                // Check for Rust macros (followed by !)
                else if self.language == "rust" && 
                        i + 1 < tokens.len() && 
                        tokens[i + 1].token_type == TokenType::Punctuation &&
                        tokens[i + 1].start < chars.len() &&
                        chars[tokens[i + 1].start] == '!' {
                    tokens[i].token_type = TokenType::Function; // Macros use function color
                    // Also mark the ! as part of the macro
                    tokens[i + 1].token_type = TokenType::Function;
                }
                // Check for function definitions - not in Markdown
                else if i > 0 && tokens[i - 1].token_type == TokenType::Keyword && self.language != "markdown" {
                    let prev_token_text: String = chars[tokens[i - 1].start..tokens[i - 1].end].iter().collect();
                    if (self.language == "rust" && prev_token_text == "fn") ||
                       (self.language == "python" && prev_token_text == "def") ||
                       (self.language == "javascript" && prev_token_text == "function") {
                        tokens[i].token_type = TokenType::Function;
                    }
                    // Check for type definitions after struct/enum in Rust
                    else if self.language == "rust" && (prev_token_text == "struct" || prev_token_text == "enum" || prev_token_text == "trait") {
                        tokens[i].token_type = TokenType::Type;
                    }
                    // Check for class definitions in Python
                    else if self.language == "python" && prev_token_text == "class" {
                        tokens[i].token_type = TokenType::Type;
                    }
                    // Check for class definitions in JavaScript/TypeScript
                    else if (self.language == "javascript" || self.language == "typescript") && prev_token_text == "class" {
                        tokens[i].token_type = TokenType::Type;
                    }
                }
                // SQL-specific: Mark AS keyword when used with type casting
                else if self.language == "sql" && i > 0 && tokens[i - 1].token_type == TokenType::Keyword {
                    let prev_token_text: String = chars[tokens[i - 1].start..tokens[i - 1].end].iter().collect();
                    if prev_token_text.to_uppercase() == "AS" {
                        // The identifier after AS is often a type in CAST expressions
                        tokens[i].token_type = TokenType::Type;
                    }
                }
            }
        }
    }
    
    /// Post-process CSS tokens to handle selectors and property names
    fn post_process_css_tokens(&self, tokens: &mut Vec<SyntaxToken>, chars: &[char]) {
        for i in 0..tokens.len() {
            if tokens[i].token_type == TokenType::Identifier {
                // Check if this identifier is followed by a colon (property name)
                let mut next_non_whitespace = i + 1;
                while next_non_whitespace < tokens.len() && 
                      tokens[next_non_whitespace].start < chars.len() &&
                      chars[tokens[next_non_whitespace].start].is_whitespace() {
                    next_non_whitespace += 1;
                }
                
                if next_non_whitespace < tokens.len() &&
                   tokens[next_non_whitespace].token_type == TokenType::Operator &&
                   tokens[next_non_whitespace].start < chars.len() &&
                   chars[tokens[next_non_whitespace].start] == ':' {
                    // This is a property name
                    tokens[i].token_type = TokenType::Property;
                }
            }
        }
    }
    
    /// Post-process JSON tokens to handle property names
    fn post_process_json_tokens(&self, tokens: &mut Vec<SyntaxToken>, chars: &[char]) {
        for i in 0..tokens.len() {
            if tokens[i].token_type == TokenType::String {
                // Check if this string is followed by a colon (property name)
                let mut next_non_whitespace = i + 1;
                while next_non_whitespace < tokens.len() && 
                      tokens[next_non_whitespace].start < chars.len() &&
                      chars[tokens[next_non_whitespace].start].is_whitespace() {
                    next_non_whitespace += 1;
                }
                
                if next_non_whitespace < tokens.len() &&
                   tokens[next_non_whitespace].token_type == TokenType::Operator &&
                   tokens[next_non_whitespace].start < chars.len() &&
                   chars[tokens[next_non_whitespace].start] == ':' {
                    // This is a property name in JSON
                    tokens[i].token_type = TokenType::Property;
                }
            }
        }
    }

    pub fn get_line_tokens(&self, line: usize) -> Option<&Vec<SyntaxToken>> {
        self.line_states.get(&line).map(|state| &state.tokens)
    }

    pub fn mark_dirty(&mut self) {
        // With the new system, we might want to mark specific lines as dirty
        // rather than clearing everything
    }

    pub fn force_update(&mut self) {
        // Force a full update by clearing the cache
        self.line_states.clear();
    }

    // Optimized update for single line changes
    pub fn update_line(&mut self, rope: &Rope, changed_line: usize) {
        if self.language == "text" {
            return;
        }

        // Get the state from the previous line
        let start_state = if changed_line > 0 {
            self.line_states.get(&(changed_line - 1))
                .map(|state| state.end_state)
                .unwrap_or(ScanState::Normal)
        } else {
            ScanState::Normal
        };

        // Scan the changed line
        let line_text = rope.line(changed_line).to_string();
        let (mut tokens, end_state) = self.scan_line(&line_text, start_state);
        
        // Post-process tokens to identify functions, types, etc.
        let chars: Vec<char> = line_text.chars().collect();
        self.post_process_tokens(&mut tokens, &chars);
        
        // Check if the end state changed
        let old_end_state = self.line_states.get(&changed_line)
            .map(|state| state.end_state);
        
        self.line_states.insert(changed_line, LineState {
            tokens,
            end_state,
        });

        // If the end state changed, we need to update subsequent lines
        if old_end_state != Some(end_state) {
            let mut current_state = end_state;
            
            for line_idx in (changed_line + 1)..rope.len_lines() {
                let line_text = rope.line(line_idx).to_string();
                let (tokens, new_end_state) = self.scan_line(&line_text, current_state);
                
                // Check if this line's end state also changed
                let old_state = self.line_states.get(&line_idx)
                    .map(|state| state.end_state);
                
                self.line_states.insert(line_idx, LineState {
                    tokens,
                    end_state: new_end_state,
                });
                
                // If the state didn't change, we can stop updating
                if old_state == Some(new_end_state) {
                    break;
                }
                
                current_state = new_end_state;
            }
        }
    }

    // Handle line insertion - shift line numbers in the hash map
    pub fn insert_line(&mut self, rope: &Rope, at_line: usize) {
        // Shift all lines at or after the insertion point
        let mut new_states = HashMap::new();
        
        for (line_idx, state) in &self.line_states {
            if *line_idx >= at_line {
                new_states.insert(line_idx + 1, state.clone());
            } else {
                new_states.insert(*line_idx, state.clone());
            }
        }
        
        self.line_states = new_states;
        
        // Now update the inserted line and any affected subsequent lines
        self.update_line(rope, at_line);
    }

    // Handle line deletion - shift line numbers and update affected lines
    pub fn delete_line(&mut self, rope: &Rope, deleted_line: usize) {
        // Remove the deleted line
        self.line_states.remove(&deleted_line);
        
        // Shift all lines after the deletion point
        let mut new_states = HashMap::new();
        
        for (line_idx, state) in &self.line_states {
            if *line_idx > deleted_line {
                new_states.insert(line_idx - 1, state.clone());
            } else {
                new_states.insert(*line_idx, state.clone());
            }
        }
        
        self.line_states = new_states;
        
        // Update the line that's now at the deleted position (if any)
        if deleted_line < rope.len_lines() {
            self.update_line(rope, deleted_line);
        }
    }
}