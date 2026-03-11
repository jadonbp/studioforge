local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local tools: { [string]: Types.ToolFunction } = {
	-- Inherited Roblox official tools
	["GetConsoleOutput"] = require(Main.Tools.GetConsoleOutput),
	["GetStudioMode"] = require(Main.Tools.GetStudioMode),
	["InsertModel"] = require(Main.Tools.InsertModel),
	["RunCode"] = require(Main.Tools.RunCode),
	["RunScriptInPlayMode"] = require(Main.Tools.RunScriptInPlayMode),
	["StartStopPlay"] = require(Main.Tools.StartStopPlay),

	-- StudioForge new tools
	["ReadScript"] = require(Main.Tools.ReadScript),
	["WriteScript"] = require(Main.Tools.WriteScript),
	["GetChildren"] = require(Main.Tools:WaitForChild("GetChildren")), -- Existing Instance.GetChildren function
	["GetProperties"] = require(Main.Tools.GetProperties),
	["GetSelection"] = require(Main.Tools.GetSelection),

	-- Phase 2: Playtest intelligence tools
	["RunClientScriptInPlayMode"] = require(Main.Tools.RunClientScriptInPlayMode),
	["GetGuiTree"] = require(Main.Tools.GetGuiTree),
	["CapturePlaytestScreenshot"] = require(Main.Tools.CapturePlaytestScreenshot),
}

local function dispatchTool(toolName: string, args: Types.ToolArgs): string?
	local toolFunction = tools[toolName]
	if not toolFunction then
		error("Tool " .. toolName .. " not found")
	end

	return toolFunction(args)
end

return {
	tools = tools,
	dispatchTool = dispatchTool,
}
