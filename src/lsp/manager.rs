// src/lsp/manager.rs

use super::{client::LspClient, LspConfig};
use crate::events::LspEvent;
use anyhow::Result;
use lsp_types::Url;
use std::collections::HashMap;

pub struct LspManager {
    clients: HashMap<String, LspClient>,
    config: LspConfig,
}

impl LspManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            clients: HashMap::new(),
            config: LspConfig::default(),
        })
    }

    pub async fn get_or_start_client(&mut self, language: &str) -> Result<&mut LspClient> {
        if !self.clients.contains_key(language) {
            let mut client = LspClient::new(language.to_string());
            
            let (server_cmd, args) = match language {
                "rust" => (&self.config.rust_analyzer_path, vec![]),
                "python" => (&self.config.python_lsp_path, vec![]),
                "javascript" => (&self.config.typescript_lsp_path, vec!["--stdio".to_string()]),
                "sql" => (&self.config.sql_lsp_path, vec![]),
                "bash" => (&self.config.bash_lsp_path, vec!["start".to_string()]),
                "xml" => (&self.config.xml_lsp_path, vec![]),
                _ => return Err(anyhow::anyhow!("Unsupported language: {}", language)),
            };

            if let Err(e) = client.start(server_cmd, &args).await {
                eprintln!("Failed to start LSP server for {}: {}", language, e);
                // Continue without LSP for this language
            }

            self.clients.insert(language.to_string(), client);
        }

        Ok(self.clients.get_mut(language).unwrap())
    }

    pub async fn did_open_text_document(&mut self, language: &str, uri: Url, text: String) -> Result<()> {
        if let Ok(client) = self.get_or_start_client(language).await {
            client.did_open_text_document(uri, text).await?;
        }
        Ok(())
    }

    pub async fn handle_event(&mut self, event: LspEvent) -> Result<()> {
        match event {
            LspEvent::Notification(method, params) => {
                // Handle LSP notifications (diagnostics, etc.)
                println!("LSP Notification: {} - {:?}", method, params);
            }
            LspEvent::Response(response) => {
                // Handle LSP responses (completion, hover, etc.)
                println!("LSP Response: {:?}", response);
            }
            LspEvent::Error(error) => {
                eprintln!("LSP Error: {}", error);
            }
        }
        Ok(())
    }
}
