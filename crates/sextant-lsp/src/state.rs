//! Backend state. Each field is wrapped in `Arc` so handler bodies can
//! cheaply hand them to `tokio::spawn`'d grading tasks without holding
//! `&self` across `.await` points.

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use sextant_core::Finding;
use sextant_engine::RuleSummary;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower_lsp::lsp_types::Url;
use tower_lsp::Client;

#[derive(Debug, Clone)]
pub(crate) struct DocumentState {
    pub(crate) text: String,
    pub(crate) version: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    /// Default `true` — LLM rules are off in the editor unless the client
    /// explicitly opts in. Mirrors `GradeOptions::no_llm`.
    pub(crate) disable_llm: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { disable_llm: true }
    }
}

#[derive(Clone)]
pub(crate) struct Backend {
    pub(crate) client: Client,
    pub(crate) docs: Arc<DashMap<Url, DocumentState>>,
    pub(crate) findings: Arc<DashMap<Url, Vec<Finding>>>,
    pub(crate) settings: Arc<RwLock<Settings>>,
    pub(crate) repo_root: Arc<RwLock<Option<PathBuf>>>,
    pub(crate) inflight: Arc<DashMap<Url, JoinHandle<()>>>,
    pub(crate) rule_cache: Arc<DashMap<String, RuleSummary>>,
}

impl Backend {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(DashMap::new()),
            findings: Arc::new(DashMap::new()),
            settings: Arc::new(RwLock::new(Settings::default())),
            repo_root: Arc::new(RwLock::new(None)),
            inflight: Arc::new(DashMap::new()),
            rule_cache: Arc::new(DashMap::new()),
        }
    }
}
