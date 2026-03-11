use crate::error::{Report, Result};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use color_eyre::eyre::{eyre, Error, OptionExt};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::oneshot::Receiver;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::Duration;
use uuid::Uuid;

pub const STUDIO_PLUGIN_PORT: u16 = 44755;
const LONG_POLL_DURATION: Duration = Duration::from_secs(15);

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ToolArguments {
    args: ToolArgumentValues,
    id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RunCommandResponse {
    success: bool,
    response: String,
    id: Uuid,
}

pub struct AppState {
    process_queue: VecDeque<ToolArguments>,
    output_map: HashMap<Uuid, mpsc::UnboundedSender<Result<String>>>,
    waiter: watch::Receiver<()>,
    trigger: watch::Sender<()>,
}
pub type PackedState = Arc<Mutex<AppState>>;

impl AppState {
    pub fn new() -> Self {
        let (trigger, waiter) = watch::channel(());
        Self {
            process_queue: VecDeque::new(),
            output_map: HashMap::new(),
            waiter,
            trigger,
        }
    }
}

impl ToolArguments {
    fn new(args: ToolArgumentValues) -> (Self, Uuid) {
        Self { args, id: None }.with_id()
    }
    fn with_id(self) -> (Self, Uuid) {
        let id = Uuid::new_v4();
        (
            Self {
                args: self.args,
                id: Some(id),
            },
            id,
        )
    }
}
#[derive(Clone)]
pub struct RBXStudioServer {
    state: PackedState,
    tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for RBXStudioServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "StudioForge".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("StudioForge MCP Server for Roblox Studio".to_string()),
                description: Some("Playtest intelligence companion for Roblox Studio's built-in MCP".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "StudioForge — Playtest intelligence companion for Roblox Studio's built-in MCP.

Provides tools the built-in Studio MCP does not offer: playtest screenshots, GUI tree inspection, client-side script execution, model insertion, and selection queries.

Check studio mode with get_studio_mode before using play-mode tools.

Playtest intelligence:
- Use run_script_in_play_mode for one-shot server-side tests (resets to stop mode after).
- Use run_client_script_in_play_mode for one-shot client-side tests (GUIs, LocalScripts, client systems).
- Use get_gui_tree to inspect the PlayerGui hierarchy during playtest.
- Use capture_playtest_screenshot to visually verify the game during playtest.

Utilities:
- Use insert_model to search and insert Creator Store models.
- Use get_selection to see what the user has selected in Explorer.
- Use get_studio_mode to check the current Studio mode.
"
                    .to_string(),
            ),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InsertModel {
    #[schemars(description = "Query to search for the model")]
    query: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetStudioMode {}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunScriptInPlayMode {
    #[schemars(description = "Code to run")]
    code: String,
    #[schemars(description = "Timeout in seconds, defaults to 100 seconds")]
    timeout: Option<u32>,
    #[schemars(description = "Mode to run in, must be start_play or run_server")]
    mode: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetSelection {}

// Phase 2: Playtest intelligence tools

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunClientScriptInPlayMode {
    #[schemars(description = "Luau code to run on the client during playtest")]
    code: String,
    #[schemars(description = "Timeout in seconds, defaults to 30")]
    timeout: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetGuiTree {
    #[schemars(description = "Maximum depth to traverse the GUI hierarchy (default 10)")]
    depth: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct CapturePlaytestScreenshot {
    #[serde(rename = "includeUI")]
    #[schemars(
        description = "Whether to include UI elements in the screenshot (default true)"
    )]
    include_ui: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
enum ToolArgumentValues {
    InsertModel(InsertModel),
    RunScriptInPlayMode(RunScriptInPlayMode),
    GetStudioMode(GetStudioMode),
    GetSelection(GetSelection),
    RunClientScriptInPlayMode(RunClientScriptInPlayMode),
    GetGuiTree(GetGuiTree),
    CapturePlaytestScreenshot(CapturePlaytestScreenshot),
}
#[tool_router]
impl RBXStudioServer {
    pub fn new(state: PackedState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Inserts a model from the Roblox marketplace into the workspace. Returns the inserted model name."
    )]
    async fn insert_model(
        &self,
        Parameters(args): Parameters<InsertModel>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::InsertModel(args))
            .await
    }

    #[tool(
        description = "Run a script in play mode and automatically stop play after script finishes or timeout. Returns the output of the script.
        Result format: { success: boolean, value: string, error: string, logs: { level: string, message: string, ts: number }[], errors: { level: string, message: string, ts: number }[], duration: number, isTimeout: boolean }.
        - Prefer using start_stop_play tool instead run_script_in_play_mode, Only used run_script_in_play_mode to run one time unit test code on server datamodel.
        - After calling run_script_in_play_mode, the datamodel status will be reset to stop mode.
        - If It returns `StudioTestService: Previous call to start play session has not been completed`, call start_stop_play tool to stop play mode first then try it again."
    )]
    async fn run_script_in_play_mode(
        &self,
        Parameters(args): Parameters<RunScriptInPlayMode>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::RunScriptInPlayMode(args))
            .await
    }

    #[tool(
        description = "Get the current studio mode. Returns the studio mode. The result will be one of start_play, run_server, or stop."
    )]
    async fn get_studio_mode(
        &self,
        Parameters(args): Parameters<GetStudioMode>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetStudioMode(args))
            .await
    }

    #[tool(
        description = "Get the currently selected instances in Roblox Studio's Explorer panel. Returns each selected instance's name, className, and path."
    )]
    async fn get_selection(
        &self,
        Parameters(args): Parameters<GetSelection>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetSelection(args))
            .await
    }

    // Phase 2: Playtest intelligence tools

    #[tool(
        description = "Run a Luau script on the CLIENT during a one-shot playtest. Useful for testing GUIs, LocalScripts, and client-side systems.
        The script runs inside the player's client context (has access to LocalPlayer, PlayerGui, UserInputService, etc.).
        Returns the same result format as run_script_in_play_mode: { success, value, error, logs, errors, duration, isTimeout }.
        Auto-resets to stop mode after the script finishes or times out.
        - Only use this when you need client-side context. For server-side tests, use run_script_in_play_mode instead.
        - If it returns 'Previous call to start play session has not been completed', call start_stop_play to stop first."
    )]
    async fn run_client_script_in_play_mode(
        &self,
        Parameters(args): Parameters<RunClientScriptInPlayMode>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::RunClientScriptInPlayMode(args))
            .await
    }

    #[tool(
        description = "Get the PlayerGui hierarchy as a JSON tree during a one-shot playtest. Returns each GUI element's name, className, visible state, position, size, text content, and children.
        Useful for inspecting UI structure, verifying element visibility, and debugging layout issues.
        Auto-resets to stop mode after capturing the tree."
    )]
    async fn get_gui_tree(
        &self,
        Parameters(args): Parameters<GetGuiTree>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetGuiTree(args))
            .await
    }

    #[tool(
        description = "Capture a screenshot of the game during a one-shot playtest and return it as an image.
        Uses CaptureService to take a screenshot from the client's perspective.
        Auto-resets to stop mode after capturing. Requires the game to be in play mode."
    )]
    async fn capture_playtest_screenshot(
        &self,
        Parameters(args): Parameters<CapturePlaytestScreenshot>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::CapturePlaytestScreenshot(args))
            .await
    }

    /// Read the most recent screenshot PNG from Roblox temp capture storage.
    /// CaptureService saves PNGs to %LOCALAPPDATA%/Roblox/tmp-capture-storage/
    /// The contentId (rbxtemp://N) doesn't map to filenames, so we find the newest file.
    fn read_screenshot_file(_content_id: &str) -> std::result::Result<String, String> {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "LOCALAPPDATA environment variable not set".to_string())?;

        let storage_dir = std::path::Path::new(&local_app_data)
            .join("Roblox")
            .join("tmp-capture-storage");

        if !storage_dir.exists() {
            return Err(format!(
                "Capture storage directory not found: {}",
                storage_dir.display()
            ));
        }

        // Find the most recently modified file in the directory
        let newest_file = std::fs::read_dir(&storage_dir)
            .map_err(|e| format!("Failed to read capture directory: {e}"))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .max_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            })
            .ok_or_else(|| "No screenshot files found in capture storage".to_string())?;

        let png_bytes = std::fs::read(newest_file.path())
            .map_err(|e| format!("Failed to read screenshot file: {e}"))?;

        Ok(BASE64.encode(&png_bytes))
    }

    async fn generic_tool_run(
        &self,
        args: ToolArgumentValues,
    ) -> Result<CallToolResult, ErrorData> {
        let (command, id) = ToolArguments::new(args);
        tracing::debug!("Running command: {:?}", command);
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<String>>();
        let trigger = {
            let mut state = self.state.lock().await;
            state.process_queue.push_back(command);
            state.output_map.insert(id, tx);
            state.trigger.clone()
        };
        trigger
            .send(())
            .map_err(|e| ErrorData::internal_error(format!("Unable to trigger send {e}"), None))?;
        let result = rx
            .recv()
            .await
            .ok_or(ErrorData::internal_error("Couldn't receive response", None))?;
        {
            let mut state = self.state.lock().await;
            state.output_map.remove_entry(&id);
        }
        tracing::debug!("Sending to MCP: {result:?}");
        match result {
            Ok(result) => {
                // Check for screenshot contentId prefixed with marker
                // Format: __screenshot__:rbxtemp://filename
                if let Some(content_id) = result.strip_prefix("__screenshot__:") {
                    match Self::read_screenshot_file(content_id) {
                        Ok(base64_data) => Ok(CallToolResult::success(vec![Content::image(
                            base64_data,
                            "image/png",
                        )])),
                        Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                            "Failed to read screenshot: {e}"
                        ))])),
                    }
                } else {
                    Ok(CallToolResult::success(vec![Content::text(result)]))
                }
            }
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }
}

pub async fn request_handler(State(state): State<PackedState>) -> Result<impl IntoResponse> {
    let timeout = tokio::time::timeout(LONG_POLL_DURATION, async {
        let mut waiter = { state.lock().await.waiter.clone() };
        loop {
            {
                let mut state = state.lock().await;
                if let Some(task) = state.process_queue.pop_front() {
                    return Ok::<ToolArguments, Error>(task);
                }
            }
            waiter.changed().await?
        }
    })
    .await;
    match timeout {
        Ok(result) => Ok(Json(result?).into_response()),
        _ => Ok((StatusCode::LOCKED, String::new()).into_response()),
    }
}

pub async fn response_handler(
    State(state): State<PackedState>,
    Json(payload): Json<RunCommandResponse>,
) -> Result<impl IntoResponse> {
    tracing::debug!("Received reply from studio {payload:?}");
    let mut state = state.lock().await;
    let tx = state
        .output_map
        .remove(&payload.id)
        .ok_or_eyre("Unknown ID")?;
    let result: Result<String, Report> = if payload.success {
        Ok(payload.response)
    } else {
        Err(Report::from(eyre!(payload.response)))
    };
    Ok(tx.send(result)?)
}

pub async fn proxy_handler(
    State(state): State<PackedState>,
    Json(command): Json<ToolArguments>,
) -> Result<impl IntoResponse> {
    let id = command.id.ok_or_eyre("Got proxy command with no id")?;
    tracing::debug!("Received request to proxy {command:?}");
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut state = state.lock().await;
        state.process_queue.push_back(command);
        state.output_map.insert(id, tx);
    }
    let result = rx.recv().await.ok_or_eyre("Couldn't receive response")?;
    {
        let mut state = state.lock().await;
        state.output_map.remove_entry(&id);
    }
    let (success, response) = match result {
        Ok(s) => (true, s),
        Err(e) => (false, e.to_string()),
    };
    tracing::debug!("Sending back to dud: success={success}, response={response:?}");
    Ok(Json(RunCommandResponse {
        success,
        response,
        id,
    }))
}

pub async fn dud_proxy_loop(state: PackedState, exit: Receiver<()>) {
    let client = reqwest::Client::new();

    let mut waiter = { state.lock().await.waiter.clone() };
    while exit.is_empty() {
        let entry = { state.lock().await.process_queue.pop_front() };
        if let Some(entry) = entry {
            let res = client
                .post(format!("http://127.0.0.1:{STUDIO_PLUGIN_PORT}/proxy"))
                .json(&entry)
                .send()
                .await;
            if let Ok(res) = res {
                let tx = {
                    state
                        .lock()
                        .await
                        .output_map
                        .remove(&entry.id.unwrap())
                        .unwrap()
                };
                let res = res
                    .json::<RunCommandResponse>()
                    .await
                    .map(|r| r.response)
                    .map_err(Into::into);
                tx.send(res).unwrap();
            } else {
                tracing::error!("Failed to proxy: {res:?}");
            };
        } else {
            waiter.changed().await.unwrap();
        }
    }
}
