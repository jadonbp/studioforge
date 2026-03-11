# StudioForge Project Guide

## Environment

This project is developed directly in Roblox Studio (no Rojo or Argon sync tool).
All script reading and writing is done through StudioForge MCP tools.

## Available Tools

### Script Tools (PRIMARY workflow)

- **read_script** — Read source code by dot-separated path. Returns source, className, lineCount.
  Example: `read_script({ path: "ServerScriptService.GameManager" })`
- **write_script** — Create or update scripts. Creates intermediate folders automatically.
  Auto-detects scriptType (Script/LocalScript/ModuleScript) from path context.
  Example: `write_script({ path: "ServerScriptService.Systems.Combat", source: "..." })`

### Exploration Tools

- **get_children** — List children of any instance by path. Returns names and classNames.
  Example: `get_children({ path: "ServerScriptService" })`
- **get_properties** — Read properties of any instance by path.
  Example: `get_properties({ path: "Workspace.SpawnLocation" })`
- **get_selection** — See what the user has selected in the Explorer panel.

### Client-Side Testing Tools

- **run_client_script_in_play_mode** — Run Luau code on the CLIENT during a one-shot playtest.
  Has access to LocalPlayer, PlayerGui, UserInputService, etc. Auto-resets to stop mode.
  Example: `run_client_script_in_play_mode({ code: "print(game.Players.LocalPlayer.Name)" })`
- **get_gui_tree** — Inspect the PlayerGui hierarchy during a one-shot playtest.
  Returns a nested JSON tree of all GUI elements with properties (visible, position, size, text).
  Example: `get_gui_tree({ depth: 5 })`
- **capture_playtest_screenshot** — Capture a screenshot of the game during playtest.
  Returns the screenshot as an image. Uses CaptureService (requires playtest).
  Example: `capture_playtest_screenshot({ includeUI: true })`

### Inherited Roblox Tools

- **run_code** — Execute Luau in edit context. Use for data queries or bulk operations.
- **get_console_output** — Read Studio output/console panel.
- **start_stop_play** — Control playtesting (start_play / stop / run_server).
- **run_script_in_play_mode** — One-shot server-side test with auto-stop. Resets to stop mode after.
- **get_studio_mode** — Check if Studio is in edit, play, or run mode.
- **insert_model** — Search and insert Creator Store models into Workspace.

## Path Format

Dot-separated, rooted at a DataModel service:
- `ServerScriptService.GameManager` — direct child
- `ServerScriptService.Systems.Combat` — nested (through Folder "Systems")
- `ReplicatedStorage.Shared.Utils.MathUtils` — deeply nested module
- `StarterPlayer.StarterPlayerScripts.ClientMain` — client script
- `Workspace.SpawnLocation` — non-script instance

## Workflows

### Reading Code
1. Use `get_children` to explore the hierarchy and discover scripts
2. Use `read_script` to read specific scripts
3. Always read before modifying

### Writing Code
1. ALWAYS `read_script` first to see the current state
2. Make targeted changes, not full rewrites unless creating new scripts
3. `write_script` automatically creates intermediate Folders for nested paths
4. After writing, verify with `read_script`

### Testing
1. Check `get_studio_mode` before starting a playtest
2. Use `start_stop_play` with mode `start_play` to enter play mode
3. Use `get_console_output` to check for errors
4. Use `start_stop_play` with mode `stop` to stop
5. Or use `run_script_in_play_mode` for automated one-shot server tests (resets to stop mode after)

### Debugging
1. Check `get_console_output` for errors and warnings
2. Use `read_script` to examine the relevant code
3. Fix with `write_script`
4. Re-test with a playtest cycle

### Visual Debugging (Client-Side)
1. Use `run_client_script_in_play_mode` to test client-side logic (GUI interactions, input handling)
2. Use `get_gui_tree` to inspect the UI hierarchy — check visibility, layout, text content
3. Use `capture_playtest_screenshot` to visually verify the game state
4. Combine: write a GUI fix → `get_gui_tree` to verify structure → `capture_playtest_screenshot` to verify visuals

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
- When in doubt, use `get_children` to verify the hierarchy before acting
