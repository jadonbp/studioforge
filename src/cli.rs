use color_eyre::eyre::{eyre, Result};
use roblox_install::RobloxStudio;
use std::env;
use std::fs;
use std::path::PathBuf;

const CLAUDE_MD_TEMPLATE: &str = include_str!("../templates/CLAUDE.md");
const MCP_JSON_TEMPLATE: &str = include_str!("../templates/mcp.json");

fn get_exe_path_string() -> Result<String> {
    let exe = env::current_exe()?;
    let path_str = exe.to_string_lossy().to_string();
    // Escape backslashes for JSON on Windows
    Ok(path_str.replace('\\', "\\\\"))
}

pub async fn init() -> Result<()> {
    let project_dir = env::current_dir()?;
    let exe_path = get_exe_path_string()?;

    // Write .mcp.json
    let mcp_json = MCP_JSON_TEMPLATE.replace("{{STUDIOFORGE_PATH}}", &exe_path);
    let mcp_path = project_dir.join(".mcp.json");
    if mcp_path.exists() {
        println!("Warning: .mcp.json already exists, overwriting.");
    }
    fs::write(&mcp_path, mcp_json)?;
    println!("[OK] Created .mcp.json");

    // Write CLAUDE.md
    let claude_md_path = project_dir.join("CLAUDE.md");
    if claude_md_path.exists() {
        println!("Warning: CLAUDE.md already exists, overwriting.");
    }
    fs::write(&claude_md_path, CLAUDE_MD_TEMPLATE)?;
    println!("[OK] Created CLAUDE.md");

    println!();
    println!("StudioForge initialized! Run `claude` in this directory to start.");
    println!("Make sure Roblox Studio is open with the StudioForge plugin enabled.");
    Ok(())
}

pub async fn doctor() -> Result<()> {
    println!("StudioForge Doctor v{}", env!("CARGO_PKG_VERSION"));
    println!("========================");

    // 1. Binary version
    println!(
        "[OK]   StudioForge binary: v{}",
        env!("CARGO_PKG_VERSION")
    );

    // 2. Plugin installed
    match check_plugin_installed() {
        Ok(path) => println!("[OK]   Studio plugin: {}", path.display()),
        Err(e) => println!("[FAIL] Studio plugin: {e}"),
    }

    // 3. Port check
    match check_port().await {
        PortStatus::Free => {
            println!("[OK]   Port 44755: available (no conflicts)")
        }
        PortStatus::InUse => {
            println!("[WARN] Port 44755: in use (another MCP instance may be running)")
        }
        PortStatus::Error(e) => {
            println!("[WARN] Port 44755: could not check ({e})")
        }
    }

    // 4. Studio connectivity
    match check_studio_connection().await {
        Ok(()) => println!("[OK]   Studio connection: plugin responding"),
        Err(e) => println!("[FAIL] Studio connection: {e}"),
    }

    // 5. CLAUDE.md
    let project_dir = env::current_dir()?;
    if project_dir.join("CLAUDE.md").exists() {
        println!("[OK]   CLAUDE.md: found");
    } else {
        println!("[WARN] CLAUDE.md: not found (run `studioforge init` to create)");
    }

    // 6. .mcp.json
    if project_dir.join(".mcp.json").exists() {
        println!("[OK]   .mcp.json: found");
    } else {
        println!(
            "[WARN] .mcp.json: not found (run `studioforge init` to create)"
        );
    }

    Ok(())
}

fn check_plugin_installed() -> Result<PathBuf> {
    let studio = RobloxStudio::locate().map_err(|_| eyre!("Roblox Studio not found"))?;
    let plugin_path = studio.plugins_path().join("MCPStudioPlugin.rbxm");
    if plugin_path.exists() {
        Ok(plugin_path)
    } else {
        Err(eyre!(
            "not installed (run `studioforge install`)"
        ))
    }
}

enum PortStatus {
    Free,
    InUse,
    Error(String),
}

async fn check_port() -> PortStatus {
    use std::net::Ipv4Addr;
    match tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 44755)).await {
        Ok(_) => PortStatus::Free,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => PortStatus::InUse,
        Err(e) => PortStatus::Error(e.to_string()),
    }
}

async fn check_studio_connection() -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    let resp = client
        .get("http://127.0.0.1:44755/request")
        .send()
        .await
        .map_err(|_| eyre!("not responding (is Roblox Studio running with MCP plugin enabled?)"))?;

    // 423 (Locked) means the server is running but no pending commands — this is success
    // 200 means there was a pending command — also success (server is running)
    if resp.status().as_u16() == 423 || resp.status().is_success() {
        Ok(())
    } else {
        Err(eyre!(
            "unexpected response: {}",
            resp.status()
        ))
    }
}
