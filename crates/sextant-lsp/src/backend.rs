//! `LanguageServer` implementation. Lifecycle methods funnel into the
//! grader (`crate::grade`) and hover handler (`crate::hover`).

use std::collections::HashMap;

use serde::Deserialize;
use sextant_engine::explain_rule;
use tower_lsp::jsonrpc::Result as JsonrpcResult;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeActionResponse, Diagnostic, DidChangeConfigurationParams,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, Hover, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, MessageType,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    WorkspaceEdit,
};
use tower_lsp::LanguageServer;

use crate::codeaction::patch_to_edits;
use crate::convert::DiagnosticData;
use crate::hover::hover_for_findings;
use crate::state::{Backend, DocumentState, Settings};
use crate::workspace::{resolve_repo_root, url_to_path};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitOptions {
    #[serde(default)]
    disable_llm: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigSettings {
    #[serde(default)]
    sextant: Option<SextantSettings>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SextantSettings {
    #[serde(default)]
    disable_llm: Option<bool>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> JsonrpcResult<InitializeResult> {
        if let Some(opts) = params.initialization_options.clone() {
            if let Ok(parsed) = serde_json::from_value::<InitOptions>(opts) {
                if let Some(disable_llm) = parsed.disable_llm {
                    self.settings.write().await.disable_llm = disable_llm;
                }
            }
        }
        let folders = params.workspace_folders.as_deref();
        let resolved = resolve_repo_root(folders, None);
        if let Some(root) = resolved {
            *self.repo_root.write().await = Some(root);
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "sextant-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "sextant-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> JsonrpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.docs.insert(
            uri.clone(),
            DocumentState {
                text: params.text_document.text,
                version: params.text_document.version,
            },
        );
        self.ensure_repo_root_from(&uri).await;
        self.schedule_grade(uri);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next_back() {
            self.docs.insert(
                uri.clone(),
                DocumentState {
                    text: change.text,
                    version: params.text_document.version,
                },
            );
        }
        self.schedule_grade(uri);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(text) = params.text {
            if let Some(mut doc) = self.docs.get_mut(&uri) {
                doc.text = text;
            }
        }
        self.grade_now(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.remove(&uri);
        self.findings.remove(&uri);
        if let Some((_, prev)) = self.inflight.remove(&uri) {
            prev.abort();
        }
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let parsed: ConfigSettings = serde_json::from_value(params.settings).unwrap_or_default();
        let disable_llm = parsed
            .sextant
            .and_then(|s| s.disable_llm)
            .unwrap_or(Settings::default().disable_llm);
        self.settings.write().await.disable_llm = disable_llm;
        self.regrade_all_open();
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        self.rule_cache.clear();
        self.regrade_all_open();
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonrpcResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let mut actions = Vec::new();
        for diag in params.context.diagnostics {
            if let Some(action) = code_action_from_diag(uri.clone(), diag) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn hover(&self, params: HoverParams) -> JsonrpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(findings) = self.findings.get(&uri).map(|f| f.clone()) else {
            return Ok(None);
        };
        let text = self.docs.get(&uri).map(|d| d.text.clone());
        let repo_root = self.repo_root.read().await.clone();
        let cache = self.rule_cache.clone();
        let lookup = |id: &str| -> Option<sextant_engine::RuleSummary> {
            if let Some(cached) = cache.get(id) {
                return Some(cached.clone());
            }
            let root = repo_root.as_deref()?;
            match explain_rule(root, id) {
                Ok(Some(rule)) => {
                    cache.insert(id.into(), rule.clone());
                    Some(rule)
                }
                _ => None,
            }
        };
        Ok(hover_for_findings(
            &findings,
            text.as_deref(),
            position,
            lookup,
        ))
    }
}

/// Build a single QuickFix `CodeAction` from one diagnostic, if its
/// `data` payload carries a Sextant patch we can apply. Returns `None`
/// when the diagnostic isn't ours, has no patch, or the patch fails to
/// parse — the editor falls back to whatever other actions are offered.
fn code_action_from_diag(uri: Url, diag: Diagnostic) -> Option<CodeAction> {
    let data = diag.data.clone()?;
    let parsed: DiagnosticData = serde_json::from_value(data).ok()?;
    let edits = patch_to_edits(&parsed.patch)?;
    let mut changes = HashMap::new();
    changes.insert(uri, edits);
    Some(CodeAction {
        title: format!("Sextant: fix {}", parsed.rule_id),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

impl Backend {
    /// If we don't have a repo root yet, walk up from the just-opened doc
    /// looking for `.sextant/` or `.git/`. Lets the LSP work in
    /// no-workspace-folder clients (e.g. `code path/to/file.rs`).
    async fn ensure_repo_root_from(&self, uri: &Url) {
        if self.repo_root.read().await.is_some() {
            return;
        }
        let Some(path) = url_to_path(uri) else { return };
        if let Some(root) = crate::workspace::walk_up_for_marker(&path) {
            *self.repo_root.write().await = Some(root);
        }
    }

    fn regrade_all_open(&self) {
        let uris: Vec<Url> = self.docs.iter().map(|e| e.key().clone()).collect();
        for uri in uris {
            self.schedule_grade(uri);
        }
    }
}
