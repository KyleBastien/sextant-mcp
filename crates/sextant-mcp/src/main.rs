//! Sextant MCP server.
//!
//! Two transports:
//!   * stdio (default) — newline-delimited JSON-RPC 2.0. **stdout is
//!     reserved for protocol traffic only**; logging goes to stderr.
//!     Closing stdin causes a clean exit.
//!   * HTTP (`--http <addr>`) — single POST endpoint at `/` that takes
//!     a JSON-RPC body and returns a JSON-RPC response. Notifications
//!     reply 202 Accepted. No streaming variants — strict req/resp.
//!
//! Both transports funnel through `handler::handle_line`, so behavior
//! is identical regardless of how the request arrived.

mod handler;
mod http;
mod protocol;
mod tools;

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;

use clap::Parser;

use crate::handler::handle_line;

#[derive(Debug, Parser)]
#[command(name = "sextant-mcp", version, about = "Sextant MCP server")]
struct Cli {
    /// Bind an HTTP server on the given address (e.g. `127.0.0.1:7331`)
    /// instead of running on stdio.
    #[arg(long, value_name = "ADDR")]
    http: Option<SocketAddr>,
}

fn main() -> std::io::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.http {
        Some(addr) => run_http(addr),
        None => run_stdio(),
    }
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

fn run_stdio() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            Ok(_) => continue,
            Err(err) => {
                tracing::error!(?err, "stdin read error; exiting");
                break;
            }
        };
        if let Some(response) = handle_line(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn run_http(addr: SocketAddr) -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(http::serve(addr))
}
