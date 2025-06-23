// src/lsp/client.rs

use anyhow::Result;
use lsp_types::*;
use std::process::Stdio;
use tokio::process::Child;

pub struct LspClient {
    pub language_id: String,
    pub server_process: Option<Child>,
    pub capabilities: Option<ServerCapabilities>,
    pub initialized: bool,
}

impl LspClient {
    pub fn new(language_id: String) -> Self {
        Self {
            language_id,
            server_process: None,
            capabilities: None,
            initialized: false,
        }
    }

    pub async fn start(&mut self, server_command: &str, args: &[String]) -> Result<()> {
        let mut cmd = tokio::process::Command::new(server_command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()?;
        self.server_process = Some(child);

        // Initialize the server
        self.send_initialize().await?;
        Ok(())
    }

    async fn send_initialize(&mut self) -> Result<()> {
        let initialize_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_path: None,
            root_uri: None,
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            initialization_options: None,
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(true),
                        will_save_wait_until: Some(false),
                        did_save: Some(true),
                    }),
                    completion: Some(CompletionClientCapabilities {
                        dynamic_registration: Some(false),
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(true),
                            commit_characters_support: Some(false),
                            documentation_format: Some(vec![MarkupKind::Markdown]),
                            deprecated_support: Some(false),
                            preselect_support: Some(false),
                            tag_support: None,
                            insert_replace_support: Some(false),
                            resolve_support: None,
                            insert_text_mode_support: None,
                            label_details_support: Some(false),
                        }),
                        completion_item_kind: None,
                        context_support: Some(false),
                        insert_text_mode: None,
                        completion_list: None,
                    }),
                    hover: Some(HoverClientCapabilities {
                        dynamic_registration: Some(false),
                        content_format: Some(vec![MarkupKind::Markdown]),
                    }),
                    signature_help: Some(SignatureHelpClientCapabilities {
                        dynamic_registration: Some(false),
                        signature_information: Some(SignatureInformationSettings {
                            documentation_format: Some(vec![MarkupKind::Markdown]),
                            parameter_information: Some(ParameterInformationSettings {
                                label_offset_support: Some(false),
                            }),
                            active_parameter_support: Some(false),
                        }),
                        context_support: Some(false),
                    }),
                    declaration: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    implementation: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    type_definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(ReferenceClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    document_highlight: Some(DocumentHighlightClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    document_symbol: Some(DocumentSymbolClientCapabilities {
                        dynamic_registration: Some(false),
                        symbol_kind: None,
                        hierarchical_document_symbol_support: Some(false),
                        tag_support: None,
                    }),
                    formatting: Some(DocumentFormattingClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    range_formatting: Some(DocumentRangeFormattingClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    on_type_formatting: Some(DocumentOnTypeFormattingClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    code_action: Some(CodeActionClientCapabilities {
                        dynamic_registration: Some(false),
                        code_action_literal_support: None,
                        is_preferred_support: Some(false),
                        disabled_support: Some(false),
                        data_support: Some(false),
                        resolve_support: None,
                        honors_change_annotations: Some(false),
                    }),
                    code_lens: Some(CodeLensClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    document_link: Some(DocumentLinkClientCapabilities {
                        dynamic_registration: Some(false),
                        tooltip_support: Some(false),
                    }),
                    color_provider: Some(DocumentColorClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    rename: Some(RenameClientCapabilities {
                        dynamic_registration: Some(false),
                        prepare_support: Some(false),
                        prepare_support_default_behavior: None,
                        honors_change_annotations: Some(false),
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        tag_support: None,
                        version_support: Some(false),
                        code_description_support: Some(false),
                        data_support: Some(false),
                    }),
                    folding_range: Some(FoldingRangeClientCapabilities {
                        dynamic_registration: Some(false),
                        range_limit: None,
                        line_folding_only: Some(false),
                        folding_range_kind: None,
                        folding_range: None,
                    }),
                    selection_range: Some(SelectionRangeClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    linked_editing_range: Some(LinkedEditingRangeClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    call_hierarchy: Some(CallHierarchyClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    semantic_tokens: Some(SemanticTokensClientCapabilities {
                        dynamic_registration: Some(false),
                        requests: SemanticTokensClientCapabilitiesRequests {
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                        token_types: vec![],
                        token_modifiers: vec![],
                        formats: vec![TokenFormat::RELATIVE],
                        overlapping_token_support: Some(false),
                        multiline_token_support: Some(false),
                        server_cancel_support: Some(false),
                        augments_syntax_tokens: Some(false),
                    }),
                    moniker: Some(MonikerClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    type_hierarchy: Some(TypeHierarchyClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    inline_value: Some(InlineValueClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    inlay_hint: Some(InlayHintClientCapabilities {
                        dynamic_registration: Some(false),
                        resolve_support: None,
                    }),
                    diagnostic: Some(DiagnosticClientCapabilities {
                        dynamic_registration: Some(false),
                        related_document_support: Some(false),
                    }),
                }),
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(false),
                    show_message: Some(ShowMessageRequestClientCapabilities {
                        message_action_item: Some(MessageActionItemCapabilities {
                            additional_properties_support: Some(false),
                        }),
                    }),
                    show_document: Some(ShowDocumentClientCapabilities {
                        support: true,
                    }),
                }),
                general: Some(GeneralClientCapabilities {
                    stale_request_support: None,
                    regular_expressions: Some(RegularExpressionsClientCapabilities {
                        engine: "ECMAScript".to_string(),
                        version: Some("ES2020".to_string()),
                    }),
                    markdown: Some(MarkdownClientCapabilities {
                        parser: "marked".to_string(),
                        version: Some("4.0.10".to_string()),
                        allowed_tags: Some(vec![]),
                    }),
                    position_encodings: Some(vec![PositionEncodingKind::UTF8]),
                }),
                workspace: Some(WorkspaceClientCapabilities {
                    apply_edit: Some(true),
                    workspace_edit: Some(WorkspaceEditClientCapabilities {
                        document_changes: Some(true),
                        resource_operations: Some(vec![
                            ResourceOperationKind::Create,
                            ResourceOperationKind::Rename,
                            ResourceOperationKind::Delete,
                        ]),
                        failure_handling: Some(FailureHandlingKind::Abort),
                        normalizes_line_endings: Some(false),
                        change_annotation_support: None,
                    }),
                    did_change_configuration: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
                        dynamic_registration: Some(false),
                        relative_pattern_support: Some(false),
                    }),
                    symbol: Some(WorkspaceSymbolClientCapabilities {
                        dynamic_registration: Some(false),
                        symbol_kind: None,
                        tag_support: None,
                        resolve_support: None,
                    }),
                    execute_command: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    workspace_folders: Some(true),
                    configuration: Some(true),
                    semantic_tokens: Some(SemanticTokensWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                    code_lens: Some(CodeLensWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                    file_operations: None,
                    inline_value: Some(InlineValueWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                    inlay_hint: Some(InlayHintWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                    diagnostic: Some(DiagnosticWorkspaceClientCapabilities {
                        refresh_support: Some(false),
                    }),
                }),
                experimental: None,
            },
            trace: Some(TraceValue::Off),
            workspace_folders: None,
            client_info: Some(ClientInfo {
                name: "tui-editor".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            locale: None,
        };

        // TODO: Send the initialize request via JSON-RPC
        // This would involve serializing the request and sending it over stdin
        // For now, we'll just mark as initialized
        self.initialized = true;
        Ok(())
    }

    pub async fn did_open_text_document(&mut self, uri: Url, text: String) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let _params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: self.language_id.clone(),
                version: 1,
                text,
            },
        };

        // TODO: Send didOpen notification
        Ok(())
    }

    pub async fn did_change_text_document(&mut self, uri: Url, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let _params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri,
                version: 1, // TODO: Track version numbers
            },
            content_changes: changes,
        };

        // TODO: Send didChange notification
        Ok(())
    }
}
