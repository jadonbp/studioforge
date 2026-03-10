use axum::routing::{get, post};
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use rbx_studio_server::*;
use rmcp::ServiceExt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{self, EnvFilter};
mod cli;
mod error;
mod install;
mod rbx_studio_server;

/// StudioForge — AI-powered development toolkit for Roblox Studio
#[derive(Parser)]
#[command(name = "studioforge", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run as MCP server on stdio (shorthand for `studioforge serve`)
    #[arg(short, long)]
    stdio: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the MCP server on stdio
    Serve,
    /// Install the Studio plugin and configure MCP clients
    Install,
    /// Generate .mcp.json and CLAUDE.md for the current project
    Init,
    /// Run diagnostic checks
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .with_target(false)
        .with_thread_ids(true)
        .init();

    let cli = Cli::parse();

    // Determine what to do based on subcommand or --stdio flag
    match cli.command {
        Some(Commands::Serve) => run_server().await,
        Some(Commands::Install) => install::install().await,
        Some(Commands::Init) => cli::init().await,
        Some(Commands::Doctor) => cli::doctor().await,
        None if cli.stdio => run_server().await,
        None => {
            // No subcommand and no --stdio flag: run install (upstream default behavior)
            install::install().await
        }
    }
}

async fn run_server() -> Result<()> {
    tracing::debug!("Debug MCP tracing enabled");

    let server_state = Arc::new(Mutex::new(AppState::new()));

    let (close_tx, close_rx) = tokio::sync::oneshot::channel();

    let listener =
        tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), STUDIO_PLUGIN_PORT)).await;

    let server_state_clone = Arc::clone(&server_state);
    let server_handle = if let Ok(listener) = listener {
        let app = axum::Router::new()
            .route("/request", get(request_handler))
            .route("/response", post(response_handler))
            .route("/proxy", post(proxy_handler))
            .with_state(server_state_clone);
        tracing::info!("This MCP instance is HTTP server listening on {STUDIO_PLUGIN_PORT}");
        tokio::spawn(async {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    _ = close_rx.await;
                })
                .await
                .unwrap();
        })
    } else {
        tracing::info!("This MCP instance will use proxy since port is busy");
        tokio::spawn(async move {
            dud_proxy_loop(server_state_clone, close_rx).await;
        })
    };

    let service = RBXStudioServer::new(Arc::clone(&server_state))
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;
    service.waiting().await?;

    close_tx.send(()).ok();
    tracing::info!("Waiting for web server to gracefully shutdown");
    server_handle.await.ok();
    tracing::info!("Bye!");
    Ok(())
}
