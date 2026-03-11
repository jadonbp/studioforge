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
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "StudioForge — AI development toolkit for Roblox Studio.

Check studio mode with get_studio_mode before using play-mode tools.

Script workflow (primary):
- Use read_script to read script source by dot-separated path (e.g., 'ServerScriptService.GameManager').
- Use write_script to create or update scripts. Creates intermediate folders automatically.
- Always read before writing. Make targeted changes, not full rewrites.

Exploration:
- Use get_children to navigate the instance hierarchy.
- Use get_properties to inspect instance properties.
- Use get_selection to see what the user has selected in Explorer.

Execution:
- Use run_code to execute Luau in edit context for queries or bulk changes.
- Use start_stop_play to control playtesting.
- Use run_script_in_play_mode for one-shot server-side tests (resets to stop mode after).
- Use get_console_output to read Studio output.

Client-side testing:
- Use run_client_script_in_play_mode for one-shot client-side tests (GUIs, LocalScripts, client systems).
- Use get_gui_tree to inspect the PlayerGui hierarchy during playtest.
- Use capture_playtest_screenshot to visually verify the game during playtest.
"
                    .to_string(),
            ),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunCode {
    #[schemars(description = "Code to run")]
    command: String,
}
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InsertModel {
    #[schemars(description = "Query to search for the model")]
    query: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetConsoleOutput {}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetStudioMode {}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct StartStopPlay {
    #[schemars(
        description = "Mode to start or stop, must be start_play, stop, or run_server. Don't use run_server unless you are sure no client/player is needed."
    )]
    mode: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunScriptInPlayMode {
    #[schemars(description = "Code to run")]
    code: String,
    #[schemars(description = "Timeout in seconds, defaults to 100 seconds")]
    timeout: Option<u32>,
    #[schemars(description = "Mode to run in, must be start_play or run_server")]
    mode: String,
}

// StudioForge new tool argument structs

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct ReadScript {
    #[schemars(description = "Dot-separated path to the script (e.g., 'ServerScriptService.GameManager')")]
    path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct WriteScript {
    #[schemars(description = "Dot-separated path for the script (e.g., 'ServerScriptService.Systems.Combat')")]
    path: String,
    #[schemars(description = "The Luau source code to write")]
    source: String,
    #[serde(rename = "scriptType")]
    #[schemars(description = "Script type: 'Script', 'LocalScript', or 'ModuleScript'. Only used when creating new scripts. Auto-detected from path if omitted.")]
    script_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetChildren {
    #[schemars(description = "Dot-separated instance path (e.g., 'ServerScriptService' or 'Workspace.Folder1')")]
    path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetProperties {
    #[schemars(description = "Dot-separated instance path (e.g., 'Workspace.SpawnLocation')")]
    path: String,
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
    RunCode(RunCode),
    InsertModel(InsertModel),
    GetConsoleOutput(GetConsoleOutput),
    StartStopPlay(StartStopPlay),
    RunScriptInPlayMode(RunScriptInPlayMode),
    GetStudioMode(GetStudioMode),
    ReadScript(ReadScript),
    WriteScript(WriteScript),
    GetChildren(GetChildren),
    GetProperties(GetProperties),
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
        description = "Runs a command in Roblox Studio and returns the printed output. Can be used to both make changes and retrieve information"
    )]
    async fn run_code(
        &self,
        Parameters(args): Parameters<RunCode>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::RunCode(args))
            .await
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

    #[tool(description = "Get the console output from Roblox Studio.")]
    async fn get_console_output(
        &self,
        Parameters(args): Parameters<GetConsoleOutput>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetConsoleOutput(args))
            .await
    }

    #[tool(
        description = "Start or stop play mode or run the server, Don't enter run_server mode unless you are sure no client/player is needed."
    )]
    async fn start_stop_play(
        &self,
        Parameters(args): Parameters<StartStopPlay>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::StartStopPlay(args))
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

    // StudioForge new tools

    #[tool(
        description = "Read a script's source code by its dot-separated instance path. Returns the source code, className, and line count."
    )]
    async fn read_script(
        &self,
        Parameters(args): Parameters<ReadScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::ReadScript(args))
            .await
    }

    #[tool(
        description = "Create or update a Script, LocalScript, or ModuleScript at the given path. Creates intermediate folders automatically. Script type is auto-detected from the path if not specified."
    )]
    async fn write_script(
        &self,
        Parameters(args): Parameters<WriteScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::WriteScript(args))
            .await
    }

    #[tool(
        description = "List the children of an instance by its dot-separated path. Returns each child's name and className."
    )]
    async fn get_children(
        &self,
        Parameters(args): Parameters<GetChildren>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetChildren(args))
            .await
    }

    #[tool(
        description = "Read properties of an instance by its dot-separated path. Returns common properties, class-specific properties, and custom attributes."
    )]
    async fn get_properties(
        &self,
        Parameters(args): Parameters<GetProperties>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetProperties(args))
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
                // Check for screenshot image data (base64 PNG prefixed with marker)
                if let Some(base64_data) = result.strip_prefix("__screenshot__:") {
                    Ok(CallToolResult::success(vec![Content::image(
                        base64_data,
                        "image/png",
                    )]))
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
