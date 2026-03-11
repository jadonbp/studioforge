local Main = script:FindFirstAncestor("MCPStudioPlugin")
local RunClientScriptInPlayMode = require(Main.Tools.RunClientScriptInPlayMode)
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")

local function buildGuiWalkerScript(depth: number): string
	local depthStr = tostring(depth)
	return [[
local Players = game:GetService("Players")
local HttpService = game:GetService("HttpService")

local MAX_DEPTH = ]] .. depthStr .. [[

local player = Players.LocalPlayer
if not player then
	player = Players.PlayerAdded:Wait()
end

local playerGui = player:WaitForChild("PlayerGui", 10)
if not playerGui then
	print("[MCP_GUI_TREE]" .. HttpService:JSONEncode({ error = "PlayerGui not found" }))
	return
end

local function serializeInstance(instance, currentDepth)
	if currentDepth > MAX_DEPTH then
		return nil
	end

	local node: any = {
		name = instance.Name,
		className = instance.ClassName,
		childCount = #instance:GetChildren(),
	}

	-- GuiObject properties (Frame, TextLabel, ImageLabel, etc.)
	if instance:IsA("GuiObject") then
		node.visible = instance.Visible

		local pos = instance.Position
		node.position = {
			xScale = pos.X.Scale,
			xOffset = pos.X.Offset,
			yScale = pos.Y.Scale,
			yOffset = pos.Y.Offset,
		}

		local size = instance.Size
		node.size = {
			xScale = size.X.Scale,
			xOffset = size.X.Offset,
			yScale = size.Y.Scale,
			yOffset = size.Y.Offset,
		}

		node.layoutOrder = instance.LayoutOrder
		node.zIndex = instance.ZIndex
	end

	-- Text properties
	if instance:IsA("TextLabel") or instance:IsA("TextButton") or instance:IsA("TextBox") then
		node.text = instance.Text
		node.textColor3 = {
			r = math.floor(instance.TextColor3.R * 255),
			g = math.floor(instance.TextColor3.G * 255),
			b = math.floor(instance.TextColor3.B * 255),
		}
		node.textSize = instance.TextSize
	end

	-- Image properties
	if instance:IsA("ImageLabel") or instance:IsA("ImageButton") then
		node.image = instance.Image
	end

	-- ScreenGui properties
	if instance:IsA("ScreenGui") then
		node.enabled = instance.Enabled
		node.displayOrder = instance.DisplayOrder
	end

	-- Recurse into children
	local children = {}
	for _, child in ipairs(instance:GetChildren()) do
		local childNode = serializeInstance(child, currentDepth + 1)
		if childNode then
			table.insert(children, childNode)
		end
	end

	if #children > 0 then
		node.children = children
	end

	return node
end

-- Build tree from PlayerGui
local tree = {
	name = "PlayerGui",
	className = "PlayerGui",
	children = {},
}

for _, child in ipairs(playerGui:GetChildren()) do
	local childNode = serializeInstance(child, 1)
	if childNode then
		table.insert(tree.children, childNode)
	end
end

tree.childCount = #tree.children

print("[MCP_GUI_TREE]" .. HttpService:JSONEncode(tree))
]]
end

local function handleGetGuiTree(args: Types.GetGuiTreeArgs): string?
	local depth = args.depth or 10

	if args.depth ~= nil then
		if type(args.depth) ~= "number" or args.depth < 1 then
			error("Depth must be a number greater than 0 in GetGuiTree")
		end
	end

	local guiWalkerCode = buildGuiWalkerScript(depth)

	-- Run the GUI walker as a client script
	local result = RunClientScriptInPlayMode({
		code = guiWalkerCode,
		timeout = 15,
	})

	if not result then
		error("No result from client script execution")
	end

	-- Parse the result to extract the GUI tree from logs
	local parsed = HttpService:JSONDecode(result)
	if parsed and parsed.logs then
		for _, log in ipairs(parsed.logs) do
			if type(log.message) == "string" and string.sub(log.message, 1, 14) == "[MCP_GUI_TREE]" then
				local treeJson = string.sub(log.message, 15)
				return treeJson
			end
		end
	end

	-- If no tree found, return the raw result for debugging
	return result
end

return handleGetGuiTree :: Types.ToolFunction
