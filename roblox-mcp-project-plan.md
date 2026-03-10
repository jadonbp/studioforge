# StudioForge — Complete Design Document (v4)
## Open-Source Roblox AI Development Toolkit

---

## 1. Vision

StudioForge is a free, open-source toolkit that makes Claude the best possible AI programming partner for Roblox development. It consists of three layers:

1. **StudioForge MCP Server** — A fork of Roblox's official `studio-rust-mcp-server` (Rust), extended with programming-focused tools. Works with Claude Code, Claude Desktop, Cursor, or any MCP client.

2. **StudioForge App** — A dedicated desktop application (Tauri/Electron) that wraps the Claude Code Agent SDK in a polished, Roblox-developer-friendly UI. Provides a stylized terminal experience with visual enhancements — screenshot feedback from Studio, conversation history, playtest status, all in one window.

3. **CLAUDE.md + CLI** — Agent instruction templates and setup tooling (`studioforge init`, `studioforge doctor`) that make Claude an effective Roblox developer regardless of which client you use.

**Design principle:** Each layer works independently. You can use just the MCP server with Claude Code CLI. You can use the app without touching a terminal. You can use the CLAUDE.md with Claude Desktop. Mix and match.

---

## 2. Architecture Overview

```
┌────────────────────────────────────────────────────────────────┐
│                    User-Facing Clients                          │
│                                                                 │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────────┐ │
│  │ Claude Code  │  │ Claude       │  │ StudioForge App       │ │
│  │ (Terminal)   │  │ Desktop      │  │ (Desktop GUI)         │ │
│  │              │  │              │  │                       │ │
│  │ Full CLI     │  │ Conversation │  │ Claude Code SDK       │ │
│  │ with diffs,  │  │ UI with MCP  │  │ wrapped in Tauri UI   │ │
│  │ git, agents, │  │ tool calls   │  │ with:                 │ │
│  │ headless     │  │              │  │ • Stylized terminal   │ │
│  │ mode, SDK    │  │              │  │ • Screenshot panel    │ │
│  │              │  │              │  │ • Playtest controls   │ │
│  │              │  │              │  │ • Connection status   │ │
│  └──────┬───────┘  └──────┬───────┘  └───────────┬───────────┘ │
│         │                  │                       │            │
│         └──────────────────┴───────────────────────┘            │
│                            │ stdio (JSON-RPC / MCP)             │
└────────────────────────────┼────────────────────────────────────┘
                             ▼
┌────────────────────────────────────────────────────────────────┐
│              StudioForge MCP Server (Rust)                      │
│              Fork of Roblox/studio-rust-mcp-server              │
│                                                                 │
│  Inherited Tools (Roblox official):                             │
│  ├── run_code              (edit-context Luau execution)        │
│  ├── insert_model          (Creator Store model insertion)      │
│  ├── get_console_output    (Studio output logs)                 │
│  ├── start_stop_play       (playtest control)                   │
│  ├── run_script_in_play_mode (server-side one-shot test)        │
│  └── get_studio_mode       (current Studio mode)                │
│                                                                 │
│  New StudioForge Tools:                                         │
│  ├── read_script           (read script source by path)         │
│  ├── write_script          (create/update scripts)              │
│  ├── run_client_script_in_play_mode (client-side one-shot test) │
│  ├── get_gui_tree          (PlayerGui hierarchy)                │
│  ├── capture_playtest_screenshot (CaptureService screenshot)    │
│  ├── get_children          (instance tree exploration)          │
│  ├── get_properties        (instance property reading)          │
│  └── get_selection         (Explorer selection)                 │
│                                                                 │
│  HTTP Bridge: localhost:33797 (127.0.0.1 only)                  │
└────────────────────────────┬────────────────────────────────────┘
                             │ HTTP (localhost, long-poll)
                             ▼
┌────────────────────────────────────────────────────────────────┐
│              Roblox Studio                                      │
│                                                                 │
│  StudioForge Plugin (Lua, extended from official)               │
│  ├── Long-poll bridge to MCP server                             │
│  ├── Handles all tool commands                                  │
│  ├── Output capture (LogService.MessageOut)                     │
│  ├── Playtest management                                        │
│  ├── Script injection for client-side play mode execution       │
│  ├── GUI tree serialization from PlayerGui                      │
│  └── CaptureService integration for screenshots                 │
└────────────────────────────────────────────────────────────────┘
```

---

## 3. Layer 1: MCP Server (Fork + Extend)

### Base: Fork of `Roblox/studio-rust-mcp-server`

Inherits the proven Rust architecture: axum HTTP server + rmcp MCP transport + long-poll plugin communication. All 6 official tools work unchanged.

### New Tool: `read_script`

**Purpose:** Read a script's source code by instance path.

**Params:** `{ path: string }` (e.g., `"ServerScriptService.GameManager"`)

**Returns:** `{ source: string, className: string, lineCount: number }`

**Plugin handler:** Uses `ScriptEditorService:GetEditorSource(instance)` to read the committed source. Walks the DataModel to resolve the path.

### New Tool: `write_script`

**Purpose:** Create or update a Script, LocalScript, or ModuleScript.

**Params:** `{ path: string, source: string, scriptType?: "Script" | "LocalScript" | "ModuleScript" }`

**Returns:** `{ success: boolean, created: boolean, path: string }`

**Plugin handler:** Uses `ScriptEditorService:UpdateSourceAsync()` to write. Creates the script instance with `Instance.new()` if it doesn't exist. Creates intermediate folders for nested paths. `scriptType` only matters when creating new scripts — defaults to `Script` for paths under ServerScriptService, `LocalScript` for paths under StarterPlayerScripts, `ModuleScript` elsewhere.

### New Tool: `run_client_script_in_play_mode`

**Purpose:** Run Luau code on the client during a one-shot playtest, mirroring the official `run_script_in_play_mode` but for client-side execution.

**Params:** `{ code: string, timeout?: number }`

**Returns:** `{ logs: string[], errors: string[], duration: number }`

**How it works:**
1. Creates a LocalScript in `StarterPlayer.StarterPlayerScripts` with the given code wrapped in output capture
2. Starts play mode via the same mechanism as `start_stop_play`
3. The LocalScript executes on the client when the player spawns
4. Captures print/warn/error output
5. Stops play after the script completes or timeout is reached
6. Cleans up the injected script
7. Returns captured output

This mirrors the exact pattern of the official `run_script_in_play_mode`, just targeting client context instead of server.

### New Tool: `capture_playtest_screenshot`

**Purpose:** Capture a screenshot of the game during playtest and return it to the AI as an image.

**Params:** `{ includeUI?: boolean }`

**Returns:** MCP image content (base64 JPEG/PNG)

**How it works (via injected client script during playtest):**
1. Injected MCPClientRunner calls `CaptureService:CaptureScreenshot()` with `UICaptureMode` set based on `includeUI`
2. The `onCaptureReady` callback receives the capture as a content ID
3. The captured image is available as a temporary asset
4. The client script reads the capture using `EditableImage` and `CaptureService` — specifically, `CaptureService:CaptureScreenshot()` returns a `contentId` that the callback receives
5. The image data gets base64-encoded and sent back through the HTTP bridge
6. The MCP server returns it as an MCP image content block

**Viability assessment:**

CaptureService is a **client-side** service that works during playtest. The key challenge is getting the image data *out* of the Roblox client and back to the MCP server. Here are the options:

**Option A: EditableImage pipeline (most promising)**
- `CaptureService:CaptureScreenshot()` triggers a capture
- The callback receives a `contentId`
- Create an `EditableImage`, use `EditableImage:DrawImage()` or pixel reading to extract raw pixel data
- Encode as base64, send back through HTTP to the MCP bridge
- This requires the EditableImage beta feature to be enabled

**Option B: File system approach**
- CaptureService can save captures to the device's gallery/filesystem
- Read the saved file from disk and send it back
- Less elegant but more reliable if EditableImage has limitations

**Option C: Deferred / Phase 2**
- If neither approach works cleanly during Studio playtest (CaptureService may behave differently in Studio vs live game), defer this to a later phase
- The GUI tree serialization approach remains the primary structural inspection tool

**Recommendation:** Prototype Option A early. If CaptureService works in Studio playtest and EditableImage can extract the data, this becomes an extremely powerful tool — Claude can literally *see* the game. If it doesn't work in Studio context, fall back to the structural approach and revisit when Roblox improves the APIs.

### New Tool: `get_gui_tree`

**Purpose:** Serialize the PlayerGui hierarchy during playtest for structural UI inspection.

**Params:** `{ depth?: number }`

**Returns:** Nested JSON tree with instance name, class, visible, position, size, text, child count.

**How it works:** Implemented as a specialized `run_client_script_in_play_mode` call that injects a GUI tree walker script. The script walks `Players.LocalPlayer.PlayerGui`, serializes each instance's key properties, and prints the JSON. The output is captured and returned.

### New Tools: `get_children`, `get_properties`, `get_selection`

Standard exploration tools. Plugin handlers query the DataModel directly in edit context. Simple implementations following the same pattern as existing tools.

---

## 4. Layer 2: StudioForge App (Desktop GUI)

### Concept

A dedicated desktop application for Roblox developers who want the power of Claude Code without living in a terminal. It wraps the **Claude Code Agent SDK** (TypeScript/Python) in a Tauri-based desktop app with a UI designed specifically for Roblox development workflows.

The app looks and feels like a modern IDE panel — a stylized terminal/chat hybrid with Roblox-specific enhancements.

### Why the Claude Code Agent SDK?

The Agent SDK gives you everything Claude Code has:
- The same agentic tool loop (read, write, edit files, run bash)
- MCP server connections (StudioForge MCP tools)
- Context management and CLAUDE.md support
- Multi-turn conversations with full history
- Subagent spawning for background tasks
- Structured output for programmatic control

But unlike the CLI, you can embed it in your own UI with:
- Custom tool approval callbacks (approve/deny in the GUI)
- Streaming response handling (show Claude thinking in real-time)
- Programmatic session management (pause, resume, branch conversations)
- Custom message injection (add screenshots, context at any point)

### UI Design

```
┌─────────────────────────────────────────────────────────────┐
│  StudioForge                                    [─] [□] [×] │
├──────────────────────────────┬──────────────────────────────┤
│                              │  Studio Status               │
│  Conversation Panel          │  ┌────────────────────────┐  │
│  (Stylized terminal look)    │  │ ● Connected (Edit)     │  │
│                              │  │ Plugin v0.2.0          │  │
│  You: Add a coin system     │  │ Project: BloxClash      │  │
│                              │  └────────────────────────┘  │
│  Claude: I'll create a       │                              │
│  coin collection system.     │  Screenshot Panel            │
│  Let me read the existing    │  ┌────────────────────────┐  │
│  code first...               │  │                        │  │
│                              │  │   [Game Screenshot]    │  │
│  > Tool: read_script         │  │                        │  │
│    ServerScriptService.Data  │  │                        │  │
│    ✓ 45 lines read           │  │                        │  │
│                              │  └────────────────────────┘  │
│  Creating CoinSystem...      │  [📸 Capture] [🔄 Refresh]  │
│                              │                              │
│  > Tool: write_script        │  Playtest Controls           │
│    ServerScriptService       │  ┌────────────────────────┐  │
│    .CoinSystem               │  │ [▶ Play] [⏹ Stop]     │  │
│    ✓ Created (34 lines)      │  │ Mode: Edit             │  │
│                              │  │ Last output: clean ✓   │  │
│  > Tool: start_stop_play     │  └────────────────────────┘  │
│    ✓ Playtest started        │                              │
│                              │  Output Panel                │
│  > Tool: get_console_output  │  ┌────────────────────────┐  │
│    [print] Spawned 10 coins  │  │ [print] Spawned 10...  │  │
│    No errors detected.       │  │ [print] Player joined  │  │
│                              │  │ [warn] Low FPS: 42     │  │
│  > Tool: capture_playtest    │  │                        │  │
│    _screenshot               │  │ Filter: [All▼]         │  │
│    [Screenshot displayed →]  │  └────────────────────────┘  │
│                              │                              │
│  The coin system is working! │  Quick Actions               │
│  ─────────────────────────── │  [Playtest & Check]          │
│  > Type a message...         │  [Read All Scripts]          │
│                              │  [Show GUI Tree]             │
├──────────────────────────────┴──────────────────────────────┤
│  StudioForge v0.2.0 | Claude Opus 4.6 | 12.4k tokens used  │
└─────────────────────────────────────────────────────────────┘
```

### Key UI Features

**Stylized Terminal Panel (Left)**
- Monospace font, dark theme, code-highlighted output
- Tool calls shown as collapsible blocks with status icons (✓, ✗, ⏳)
- File diffs shown inline when `write_script` is called (computed by comparing read → write)
- Approve/deny buttons for tool calls (using SDK's approval callback)
- Streaming response — text appears as Claude thinks

**Screenshot Panel (Right)**
- Displays the latest `capture_playtest_screenshot` result
- Manual capture button triggers a screenshot on demand
- Before/after comparison when verifying visual changes
- Can be toggled to show GUI tree visualization instead

**Playtest Controls (Right)**
- One-click play/stop buttons that call `start_stop_play`
- Live status indicator (Edit/Playing/Paused)
- Connection health dot (green/yellow/red)

**Output Panel (Right)**
- Live feed from `get_console_output`, filterable by level
- Clickable errors that navigate Claude to investigate

**Quick Actions (Right)**
- Pre-built prompt buttons for common workflows
- "Playtest & Check" = start playtest, wait, get output, stop, report
- "Read All Scripts" = enumerate and read the project's scripts
- "Show GUI Tree" = playtest + get_gui_tree + display

### Tech Stack for the App

| Component | Technology | Why |
|---|---|---|
| Desktop framework | Tauri v2 | Lightweight (Rust backend, web frontend), native feel, small binary |
| Frontend | React + Tailwind | Fast iteration, great component ecosystem |
| Claude integration | Claude Code Agent SDK (TypeScript) | Same agentic capabilities as Claude Code CLI |
| MCP connection | Spawns StudioForge MCP server as child process | Same as Claude Code does |
| Image display | Native HTML `<img>` with base64 | CaptureService screenshots displayed inline |

### How It Connects

```
StudioForge App (Tauri)
├── Tauri backend (Rust)
│   └── Spawns: rbx-studio-mcp --stdio (StudioForge MCP server)
├── Frontend (React)
│   └── Uses: @anthropic-ai/claude-code SDK (TypeScript)
│       ├── Connects to MCP server via stdio
│       ├── Sends user prompts
│       ├── Receives streaming responses
│       ├── Handles tool approval callbacks → renders approve/deny buttons
│       └── Displays results (text, images, tool outputs)
└── IPC between Tauri backend and frontend for native operations
```

The Agent SDK handles the Claude conversation loop. The MCP server handles Studio communication. The Tauri app renders it all in a beautiful UI.

---

## 5. Layer 3: CLAUDE.md + CLI

### `studioforge init`

Detects the project environment (Rojo, Argon, plain Studio) and generates:

- `.mcp.json` — MCP server configuration for Claude Code
- `CLAUDE.md` — Tailored agent instructions with project-specific context

If Rojo/Argon is detected, the CLAUDE.md instructs Claude to prefer editing `.lua` files natively (full diffs). If no sync tool is detected, it instructs Claude to use `write_script`/`read_script` MCP tools.

### `studioforge doctor`

Diagnostic tool that checks:
- MCP server binary exists and runs
- Port 33797 is available
- Studio plugin is installed and responding (heartbeat check)
- Sync tool running (if applicable)
- CLAUDE.md exists in project root

### CLAUDE.md Template

The comprehensive Roblox development guide we designed — conventions, workflows, error patterns, safety rules. Adapted based on project detection:

```markdown
# Project: {name}

## Environment
{sync tool context OR "Scripts are managed via StudioForge MCP tools"}

## StudioForge Tools Available
{tool list with descriptions}

## Roblox Development Conventions
{architecture rules, Luau idioms, common patterns}

## Workflows
{writing features, debugging, testing, verifying UI}

## Safety Rules
{read before write, explain before changing, small increments}
```

---

## 6. CaptureService Deep Dive

### What CaptureService Can Do

CaptureService is a **client-side service** that programmatically captures screenshots during gameplay. Key methods:

- `CaptureScreenshot(onCaptureReady, captureParams)` — Takes a screenshot, fires callback with the content ID
- `captureParams` can include `UICaptureMode` (None, IncludeUI) to control whether GUI elements appear
- The callback receives a `contentId` pointing to the captured image

### The Pipeline: Screenshot → AI

The challenge is getting image bytes from Roblox's content system to the MCP server. Here's the most viable pipeline:

```
1. MCPClientRunner calls CaptureService:CaptureScreenshot()
2. Callback receives contentId
3. Create EditableImage from the capture
4. Read pixels with EditableImage:ReadPixelsBuffer()
5. Encode as base64 (in Lua, using a base64 module)
6. Send the base64 string back through HTTP to MCP bridge
7. MCP server wraps it as an MCP image content block
8. Claude receives and can "see" the game
```

### Considerations

- **EditableImage is required** for pixel extraction — this is a relatively new API and may have Studio-specific limitations
- **Image size matters** — full resolution screenshots could be large. Downscaling to ~800x600 before encoding keeps the payload manageable
- **Playtest only** — CaptureService is a client-side service, so screenshots can only be taken during active playtest
- **Studio vs. live game** — CaptureService behavior in Studio playtest needs to be validated (it should work since Studio simulates a full client, but edge cases are possible)
- **Claude's vision capabilities** — Claude can analyze images natively. A screenshot + the prompt "does this UI look correct?" is incredibly powerful for visual debugging

### Fallback: OS-Level Capture

If CaptureService doesn't work in Studio context, kevinswint's fork demonstrated OS-level window capture (using platform-specific APIs on macOS/Windows to screenshot the Studio window). This is less elegant but platform-proven. The Rust MCP server could handle this directly without needing the plugin.

---

## 7. Client Compatibility Matrix

| Client | How It Works | Features |
|---|---|---|
| **Claude Code CLI** | `claude mcp add studioforge -- ./rbx-studio-mcp --stdio` | Full CLI power: diffs (with Rojo), git, agents, headless mode, subagents |
| **Claude Code Headless** | `claude -p "fix the coin bug" --mcp studioforge` | Automated pipelines, CI-like workflows, batch operations |
| **Claude Desktop** | Add to `claude_desktop_config.json` MCP servers | Conversation UI with tool calls, good for exploration |
| **StudioForge App** | Built-in, launches MCP server automatically | Best experience: screenshots, playtest controls, visual output |
| **Cursor** | Add to MCP config | Inline code editing with Studio access |
| **Any MCP client** | Point to `rbx-studio-mcp --stdio` | Base MCP tool access |

### Claude Code Agent SDK Benefits (for the App)

The SDK gives StudioForge App capabilities beyond what any MCP client offers:

- **Custom tool rendering** — Screenshot results display as images, not JSON blobs
- **Approval workflows** — "Claude wants to write_script. [Approve] [Deny] [View Diff]"
- **Streaming** — See Claude's reasoning in real-time as it works
- **Session management** — Pause a conversation, branch it, resume later
- **Subagents** — "Have a background agent run all playtests while I continue chatting"
- **Structured output** — Parse tool results into native UI components
- **Custom system prompts** — Inject the CLAUDE.md content programmatically plus real-time Studio state

---

## 8. Development Phases

### Phase 1: Fork + Core Tools (Weeks 1-3)

- [ ] Fork `Roblox/studio-rust-mcp-server`
- [ ] Study existing architecture (Rust MCP server + Lua plugin)
- [ ] Add `read_script` tool (simplest extension, proves the pattern)
- [ ] Add `write_script` tool (core programming capability)
- [ ] Add `get_children`, `get_properties`, `get_selection`
- [ ] Write CLAUDE.md template
- [ ] Build `studioforge init` and `studioforge doctor` CLI commands
- [ ] Test end-to-end with Claude Code CLI
- [ ] **Ship as v0.1.0** — already more useful than any free alternative

### Phase 2: Playtest Intelligence (Weeks 4-6)

- [ ] Add `run_client_script_in_play_mode` (mirrors official server-side pattern)
- [ ] Add `get_gui_tree` (via client script injection)
- [ ] Prototype CaptureService screenshot pipeline
- [ ] Add `capture_playtest_screenshot` if viable
- [ ] Expanded CLAUDE.md with testing and debugging workflows
- [ ] **Ship as v0.2.0**

### Phase 3: StudioForge App (Weeks 7-12)

- [ ] Tauri project scaffolding
- [ ] Claude Code Agent SDK integration (TypeScript)
- [ ] Conversation UI with stylized terminal look
- [ ] Tool call rendering (collapsible blocks, status icons)
- [ ] Screenshot panel (display CaptureService images inline)
- [ ] Playtest control panel
- [ ] Output feed panel
- [ ] Quick action buttons
- [ ] Approval/deny workflow for tool calls
- [ ] **Ship as v0.3.0** (beta)

### Phase 4: Polish + Community (Ongoing)

- [ ] Creator Store plugin publication
- [ ] Documentation site
- [ ] Demo videos and tutorials
- [ ] Community CLAUDE.md extensions (genre-specific patterns)
- [ ] Contribution guidelines
- [ ] App auto-update mechanism

---

## 9. Security Model

| Principle | Implementation |
|---|---|
| **Fully local** | HTTP bound to 127.0.0.1. No external connections from MCP server or plugin. |
| **Zero telemetry** | No analytics, no crash reporting. Verifiable in source. |
| **No accounts** | No signup or API keys for StudioForge. Claude auth is handled by the user's own Claude subscription. |
| **Open source** | MIT license. Full source available. |
| **Minimal surface** | ~14 tools total. Fork stays close to upstream. |
| **Screenshots stay local** | CaptureService images are base64-encoded in memory, never saved to disk or uploaded anywhere except to Claude's context. |

---

## 10. Why This Wins

**For the Roblox developer who just wants to build games:**
→ Install plugin, install app, start talking. No terminal, no Rojo, no git required.

**For the programmer who lives in the terminal:**
→ Claude Code CLI with full diffs (via Rojo), git integration, headless mode for automation.

**For everyone:**
→ The CLAUDE.md makes Claude actually understand Roblox. The playtest loop runs autonomously. Screenshots let Claude see what it built. All of it free, local, and open source.

**Versus RoPilot:** Free vs $20-250/mo. Open source vs closed. Local vs metered. Universal client support vs Claude-only.

**Versus existing open-source:** Programming-focused. Ships with agent instructions. Screenshot feedback. Dedicated app experience. One-command setup.
