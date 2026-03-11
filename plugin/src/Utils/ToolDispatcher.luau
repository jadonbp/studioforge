local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local tools: { [string]: Types.ToolFunction } = {
	-- Utilities
	["GetStudioMode"] = require(Main.Tools.GetStudioMode),
	["InsertModel"] = require(Main.Tools.InsertModel),
	["GetSelection"] = require(Main.Tools.GetSelection),

	-- Playtest intelligence
	["RunScriptInPlayMode"] = require(Main.Tools.RunScriptInPlayMode),
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
