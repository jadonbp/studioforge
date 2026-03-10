local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)
local PathResolver = require(Main.Utils.PathResolver)
local HttpService = game:GetService("HttpService")

-- Class-specific readable properties
local CLASS_PROPERTIES: { [string]: { string } } = {
	BasePart = { "Position", "Size", "Color", "Material", "Anchored", "CanCollide", "Transparency", "CFrame" },
	Script = { "Enabled", "RunContext" },
	LocalScript = { "Enabled" },
	ModuleScript = {},
	Model = { "PrimaryPart", "WorldPivot" },
	Folder = {},
	ScreenGui = { "Enabled", "DisplayOrder", "ResetOnSpawn", "IgnoreGuiInset" },
	Frame = { "Position", "Size", "BackgroundColor3", "BackgroundTransparency", "Visible", "LayoutOrder", "ZIndex" },
	TextLabel = { "Text", "TextSize", "TextColor3", "Font", "Position", "Size", "Visible", "BackgroundTransparency" },
	TextButton = { "Text", "TextSize", "TextColor3", "Font", "Position", "Size", "Visible" },
	TextBox = { "Text", "PlaceholderText", "TextSize", "Position", "Size", "Visible" },
	ImageLabel = { "Image", "ImageColor3", "Position", "Size", "Visible", "ScaleType" },
	ImageButton = { "Image", "Position", "Size", "Visible" },
	UIListLayout = { "FillDirection", "HorizontalAlignment", "VerticalAlignment", "Padding", "SortOrder" },
	UIGridLayout = { "CellSize", "CellPadding", "FillDirection", "SortOrder" },
	UIPadding = { "PaddingTop", "PaddingBottom", "PaddingLeft", "PaddingRight" },
	UICorner = { "CornerRadius" },
	Sound = { "SoundId", "Volume", "Playing", "Looped", "PlaybackSpeed" },
	RemoteEvent = {},
	RemoteFunction = {},
	BindableEvent = {},
	Camera = { "CFrame", "FieldOfView", "CameraType" },
	SpawnLocation = { "Enabled", "TeamColor", "AllowTeamChangeOnTouch" },
	Humanoid = { "Health", "MaxHealth", "WalkSpeed", "JumpPower", "JumpHeight" },
}

-- Inheritance mappings for IsA checks
local INHERITANCE_CHECKS = { "BasePart" }

local function safeToString(value: any): string
	local ok, result = pcall(tostring, value)
	return if ok then result else "<<unreadable>>"
end

local function handleGetProperties(args: Types.GetPropertiesArgs): string?
	if type(args.path) ~= "string" or args.path == "" then
		error("Missing or empty 'path' parameter")
	end

	local instance = PathResolver.resolvePath(args.path)
	if not instance then
		error("Instance not found at path: " .. args.path)
	end

	local props: { [string]: string } = {}

	-- Common properties
	props["Name"] = instance.Name
	props["ClassName"] = instance.ClassName
	props["Parent"] = if instance.Parent then instance.Parent:GetFullName() else "nil"

	-- Class-specific properties (exact class match)
	local classProps = CLASS_PROPERTIES[instance.ClassName]
	if classProps then
		for _, propName in ipairs(classProps) do
			local ok, value = pcall(function()
				return (instance :: any)[propName]
			end)
			if ok then
				props[propName] = safeToString(value)
			end
		end
	end

	-- Inheritance-based properties
	for _, baseClass in ipairs(INHERITANCE_CHECKS) do
		if instance:IsA(baseClass) and instance.ClassName ~= baseClass then
			local baseProps = CLASS_PROPERTIES[baseClass]
			if baseProps then
				for _, propName in ipairs(baseProps) do
					if not props[propName] then
						local ok, value = pcall(function()
							return (instance :: any)[propName]
						end)
						if ok then
							props[propName] = safeToString(value)
						end
					end
				end
			end
		end
	end

	-- Custom attributes
	local attributes = instance:GetAttributes()
	if next(attributes) then
		local attrStrings: { [string]: string } = {}
		for key, value in pairs(attributes) do
			attrStrings[key] = safeToString(value)
		end
		props["_attributes"] = HttpService:JSONEncode(attrStrings)
	end

	-- Child count (useful context)
	props["_childCount"] = tostring(#instance:GetChildren())

	return HttpService:JSONEncode(props)
end

return handleGetProperties :: Types.ToolFunction
