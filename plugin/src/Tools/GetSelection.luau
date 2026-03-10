local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)
local HttpService = game:GetService("HttpService")
local Selection = game:GetService("Selection")

local function handleGetSelection(_args: Types.GetSelectionArgs): string?
	local selected = Selection:Get()

	local results = {}
	for _, instance in ipairs(selected) do
		local fullName = instance:GetFullName()
		-- Remove leading "game." prefix for consistency with our path format
		local path = string.gsub(fullName, "^game%.", "")

		table.insert(results, {
			name = instance.Name,
			className = instance.ClassName,
			path = path,
		})
	end

	return HttpService:JSONEncode({
		count = #results,
		selection = results,
	})
end

return handleGetSelection :: Types.ToolFunction
