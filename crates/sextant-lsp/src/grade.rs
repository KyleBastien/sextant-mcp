//! Per-URI debounced grading. On every `did_change` we cancel any pending
//! grade for the same URI, sleep, then run the engine on a blocking thread
//! and publish diagnostics. Cancellation prevents a burst of keystrokes
//! from queuing up redundant grades.

use std::path::PathBuf;
use std::time::Duration;

use sextant_core::SourceFile;
use sextant_engine::{grade_file_buffer, GradeOptions};
use tower_lsp::lsp_types::{Diagnostic, Url};

use crate::convert::finding_to_diagnostic;
use crate::state::Backend;

const DEBOUNCE: Duration = Duration::from_millis(400);

impl Backend {
    /// Cancel any pending grade for `uri` and queue a new debounced one.
    pub(crate) fn schedule_grade(&self, uri: Url) {
        if let Some((_, prev)) = self.inflight.remove(&uri) {
            prev.abort();
        }
        let backend = self.clone();
        let task_uri = uri.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(DEBOUNCE).await;
            backend.run_grade(task_uri.clone()).await;
            backend.inflight.remove(&task_uri);
        });
        self.inflight.insert(uri, handle);
    }

    /// Grade `uri` immediately (no debounce). Used by `did_save` so a save
    /// publishes diagnostics without waiting on the debounce window.
    pub(crate) async fn grade_now(&self, uri: Url) {
        if let Some((_, prev)) = self.inflight.remove(&uri) {
            prev.abort();
        }
        self.run_grade(uri).await;
    }

    async fn run_grade(&self, uri: Url) {
        let Some(repo_root) = self.repo_root.read().await.clone() else {
            tracing::debug!(%uri, "no repo root resolved; skipping grade");
            return;
        };
        let Some(doc) = self.docs.get(&uri).map(|d| d.clone()) else {
            return;
        };
        let Some(path) = uri.to_file_path().ok() else {
            tracing::debug!(%uri, "non-file URI; skipping grade");
            return;
        };
        let opts = GradeOptions {
            no_llm: self.settings.read().await.disable_llm,
        };
        let report = match run_engine(repo_root, path, doc.text, opts).await {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!(?err, %uri, "engine error");
                return;
            }
        };
        let text = self.docs.get(&uri).map(|d| d.text.clone());
        let diagnostics: Vec<Diagnostic> = report
            .findings
            .iter()
            .map(|f| finding_to_diagnostic(f, text.as_deref()))
            .collect();
        self.findings.insert(uri.clone(), report.findings);
        self.client
            .publish_diagnostics(uri, diagnostics, Some(doc.version))
            .await;
    }
}

async fn run_engine(
    repo_root: PathBuf,
    path: PathBuf,
    text: String,
    opts: GradeOptions,
) -> Result<sextant_core::Report, anyhow::Error> {
    tokio::task::spawn_blocking(move || {
        grade_file_buffer(&repo_root, SourceFile::new(path, text), opts)
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    //! `Backend` requires a real `Client`, which requires a connected LSP
    //! transport — so we don't unit-test scheduling here. End-to-end
    //! coverage lives in the VS Code extension's manual smoke test
    //! (see docs/editor/vscode.mdx).
    use super::*;

    #[test]
    fn debounce_is_under_one_second() {
        assert!(DEBOUNCE < Duration::from_secs(1));
    }
}
