local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)
local PathResolver = require(Main.Utils.PathResolver)
local HttpService = game:GetService("HttpService")
local ScriptEditorService = game:GetService("ScriptEditorService")

-- Default script types based on the containing service
local SCRIPT_TYPE_DEFAULTS: { [string]: string } = {
	ServerScriptService = "Script",
	ServerStorage = "ModuleScript",
	ReplicatedStorage = "ModuleScript",
	StarterPlayerScripts = "LocalScript",
	StarterCharacterScripts = "LocalScript",
	StarterGui = "LocalScript",
	StarterPack = "LocalScript",
}

local function detectScriptType(parts: { string }): string
	local service = parts[1]

	-- Special handling for StarterPlayer subcontainers
	if service == "StarterPlayer" and #parts >= 2 then
		local subContainer = parts[2]
		local subDefault = SCRIPT_TYPE_DEFAULTS[subContainer]
		if subDefault then
			-- Direct children of the subcontainer get the subcontainer's default
			-- Nested children become ModuleScript (they are required by the entry point)
			if #parts == 3 then
				return subDefault
			else
				return "ModuleScript"
			end
		end
	end

	local default = SCRIPT_TYPE_DEFAULTS[service] or "ModuleScript"

	-- Direct children of ServerScriptService are Scripts (entry points)
	-- Nested children are ModuleScripts (they are required by the entry point)
	if service == "ServerScriptService" and #parts > 2 then
		return "ModuleScript"
	end

	-- Direct children of client containers are LocalScripts
	-- Nested children are ModuleScripts
	if (service == "StarterGui" or service == "StarterPack") and #parts > 2 then
		return "ModuleScript"
	end

	return default
end

local function ensureIntermediateFolders(parts: { string }): Instance
	-- parts[1] is the service, parts[#parts] is the script name
	-- parts[2..#parts-1] are intermediate folders

	local ok, current = pcall(function()
		return game:GetService(parts[1])
	end)
	if not ok or not current then
		error("Service not found: " .. parts[1])
	end

	-- For StarterPlayer paths, the second part might be a known child (not a folder to create)
	local startIdx = 2

	for i = startIdx, #parts - 1 do
		local child = current:FindFirstChild(parts[i])
		if not child then
			local folder = Instance.new("Folder")
			folder.Name = parts[i]
			folder.Parent = current
			child = folder
		end
		current = child
	end

	return current
end

local function handleWriteScript(args: Types.WriteScriptArgs): string?
	if type(args.path) ~= "string" or args.path == "" then
		error("Missing or empty 'path' parameter")
	end
	if type(args.source) ~= "string" then
		error("Missing 'source' parameter")
	end

	local parts = string.split(args.path, ".")
	if #parts < 2 then
		error("Path must include at least a service and script name (e.g., 'ServerScriptService.MyScript')")
	end

	-- Check if script already exists
	local existing = PathResolver.resolvePath(args.path)

	if existing then
		-- Update existing script
		if not existing:IsA("LuaSourceContainer") then
			error(
				"Instance at '"
					.. args.path
					.. "' is not a script (found "
					.. existing.ClassName
					.. "). Cannot update source."
			)
		end

		ScriptEditorService:UpdateSourceAsync(existing, function(_oldSource)
			return args.source
		end)

		return HttpService:JSONEncode({
			success = true,
			created = false,
			path = args.path,
			className = existing.ClassName,
		})
	end

	-- Create new script
	local scriptType = args.scriptType
	if not scriptType or scriptType == "" then
		scriptType = detectScriptType(parts)
	end

	-- Validate scriptType
	if scriptType ~= "Script" and scriptType ~= "LocalScript" and scriptType ~= "ModuleScript" then
		error("Invalid scriptType: " .. tostring(scriptType) .. ". Must be Script, LocalScript, or ModuleScript.")
	end

	-- Ensure intermediate folders exist
	local parent = ensureIntermediateFolders(parts)
	local scriptName = parts[#parts]

	-- Check if something with this name already exists at the target
	local existingChild = parent:FindFirstChild(scriptName)
	if existingChild then
		error(
			"An instance named '"
				.. scriptName
				.. "' already exists at this path but is a "
				.. existingChild.ClassName
				.. ", not a script."
		)
	end

	-- Create the script instance
	local newScript = Instance.new(scriptType)
	newScript.Name = scriptName
	newScript.Parent = parent

	-- Write source using ScriptEditorService
	ScriptEditorService:UpdateSourceAsync(newScript, function(_oldSource)
		return args.source
	end)

	return HttpService:JSONEncode({
		success = true,
		created = true,
		path = args.path,
		className = scriptType,
	})
end

return handleWriteScript :: Types.ToolFunction
