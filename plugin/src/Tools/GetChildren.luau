local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)
local PathResolver = require(Main.Utils.PathResolver)
local HttpService = game:GetService("HttpService")

local function handleGetChildren(args: Types.GetChildrenArgs): string?
	if type(args.path) ~= "string" or args.path == "" then
		error("Missing or empty 'path' parameter")
	end

	local instance = PathResolver.resolvePath(args.path)
	if not instance then
		error("Instance not found at path: " .. args.path)
	end

	local children = {}
	for _, child in ipairs(instance:GetChildren()) do
		table.insert(children, {
			name = child.Name,
			className = child.ClassName,
		})
	end

	return HttpService:JSONEncode({
		path = args.path,
		className = instance.ClassName,
		childCount = #children,
		children = children,
	})
end

return handleGetChildren :: Types.ToolFunction
