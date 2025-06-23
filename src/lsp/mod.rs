// src/lsp/mod.rs

mod client;
mod manager;

pub use manager::LspManager;

pub struct LspConfig {
    pub rust_analyzer_path: String,
    pub python_lsp_path: String,
    pub typescript_lsp_path: String,
    pub sql_lsp_path: String,
    pub bash_lsp_path: String,
    pub xml_lsp_path: String,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            rust_analyzer_path: "rust-analyzer".to_string(),
            python_lsp_path: "pylsp".to_string(),
            typescript_lsp_path: "typescript-language-server".to_string(),
            sql_lsp_path: "sqls".to_string(),
            bash_lsp_path: "bash-language-server".to_string(),
            xml_lsp_path: "lemminx".to_string(),
        }
    }
}
