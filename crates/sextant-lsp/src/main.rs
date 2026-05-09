//! Sextant LSP server.
//!
//! Stdio-only LSP that drives `sextant-engine::grade_file_buffer` against
//! every open document with debouncing, plus `explain_rule` for hover
//! popovers. Stdout is reserved for protocol traffic; logs go to stderr.

mod backend;
mod convert;
mod grade;
mod hover;
mod state;
mod workspace;

use clap::Parser;
use tower_lsp::{LspService, Server};

use crate::state::Backend;

#[derive(Debug, Parser)]
#[command(name = "sextant-lsp", version, about = "Sextant LSP server")]
struct Cli {
    /// Accepted for compatibility with clients that pass `--stdio` to
    /// every language server. Stdio is the only transport.
    #[arg(long, default_value_t = false)]
    stdio: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    init_tracing();
    let _cli = Cli::parse();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();
}
