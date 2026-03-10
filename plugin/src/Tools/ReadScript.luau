local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)
local PathResolver = require(Main.Utils.PathResolver)
local HttpService = game:GetService("HttpService")
local ScriptEditorService = game:GetService("ScriptEditorService")

local function handleReadScript(args: Types.ReadScriptArgs): string?
	if type(args.path) ~= "string" or args.path == "" then
		error("Missing or empty 'path' parameter")
	end

	local instance = PathResolver.resolvePath(args.path)
	if not instance then
		error("Script not found at path: " .. args.path)
	end

	if not instance:IsA("LuaSourceContainer") then
		error("Instance at '" .. args.path .. "' is not a script (found " .. instance.ClassName .. ")")
	end

	local source = ScriptEditorService:GetEditorSource(instance)

	-- Count lines
	local lineCount = 1
	for _ in string.gmatch(source, "\n") do
		lineCount += 1
	end

	return HttpService:JSONEncode({
		path = args.path,
		className = instance.ClassName,
		source = source,
		lineCount = lineCount,
	})
end

return handleReadScript :: Types.ToolFunction
