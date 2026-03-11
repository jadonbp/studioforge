# StudioForge — Playtest Intelligence Companion

## Overview

StudioForge is a companion MCP server that extends Roblox Studio's built-in MCP with playtest intelligence tools. The built-in Studio MCP handles script editing, exploration, and input simulation. StudioForge adds visual debugging, GUI inspection, and programmatic playtest execution.

## StudioForge Tools

### Playtest Intelligence

- **run_script_in_play_mode** — One-shot server-side test with structured results. Injects a Script, runs a playtest, captures output, and auto-stops. Returns `{ success, value, error, logs, errors, duration, isTimeout }`.
  Example: `run_script_in_play_mode({ code: "print('hello')", mode: "start_play" })`

- **run_client_script_in_play_mode** — One-shot CLIENT-side test. Runs Luau in the player's client context (LocalPlayer, PlayerGui, UserInputService). Auto-stops after.
  Example: `run_client_script_in_play_mode({ code: "print(game.Players.LocalPlayer.Name)" })`

- **get_gui_tree** — Inspect the PlayerGui hierarchy during a one-shot playtest. Returns a nested JSON tree with name, className, visible, position, size, text, and children.
  Example: `get_gui_tree({ depth: 5 })`

- **capture_playtest_screenshot** — Capture a screenshot of the game during playtest. Returns the image directly. Uses CaptureService.
  Example: `capture_playtest_screenshot({ includeUI: true })`

### Utilities

- **insert_model** — Search and insert Creator Store models into Workspace.
  Example: `insert_model({ query: "sword" })`
- **get_selection** — See what the user has selected in the Explorer panel.
- **get_studio_mode** — Check current Studio mode (start_play / run_server / stop).

## Workflows

### Visual Debugging
1. Write code using the built-in Studio MCP's `multi_edit`
2. Use `run_client_script_in_play_mode` to test client-side logic
3. Use `get_gui_tree` to verify UI structure
4. Use `capture_playtest_screenshot` to visually verify the game state

### Testing
1. Check `get_studio_mode` before starting a playtest
2. Use `run_script_in_play_mode` for server-side tests (auto-stops)
3. Use `run_client_script_in_play_mode` for client-side tests (auto-stops)
4. Both return structured results with logs, errors, and timing

## Roblox Conventions

### Service Layout
- **ServerScriptService** — Server-side Scripts (run on server only)
- **ServerStorage** — Server-only assets and ModuleScripts
- **ReplicatedStorage** — Shared ModuleScripts (accessible by both server and client)
- **StarterPlayer.StarterPlayerScripts** — Client-side LocalScripts
- **StarterPlayer.StarterCharacterScripts** — Per-character LocalScripts
- **StarterGui** — Client UI (ScreenGuis with LocalScripts)
- **Workspace** — 3D world objects (Parts, Models, etc.)

### Luau Idioms
- Use `ModuleScript` for shared/reusable logic, `require()` to import
- Server-client communication: `RemoteEvent` and `RemoteFunction` in ReplicatedStorage
- Never trust the client: validate all remote calls on the server
- Use `task.spawn()` / `task.defer()` instead of raw coroutines
- Use `task.wait()` instead of deprecated `wait()`
- Use `local` for all variables (no globals)
- Type annotations: `function foo(x: number): string`

### Common Patterns
- Entry point Scripts in ServerScriptService that `require()` ModuleScripts
- Service pattern: ModuleScripts in ServerStorage/ReplicatedStorage that return tables
- Event-driven architecture using Roblox signals (`.Changed`, `.Touched`, etc.)
- DataStore for persistence (server-side only)

## Safety Rules

- Always read before writing
- Explain proposed changes before making them
- Make small, incremental changes
- Test after each significant change
- Never delete scripts without user confirmation
