--[[
	PathResolver — Shared utility for resolving dot-separated instance paths.

	Path format: "ServiceName.Child.GrandChild"
	Examples:
	  "ServerScriptService.GameManager"
	  "ReplicatedStorage.Shared.Utils.MathUtils"
	  "Workspace.SpawnLocation"
]]

local function resolvePath(path: string): Instance?
	local parts = string.split(path, ".")
	if #parts == 0 then
		return nil
	end

	-- First part is a service name
	local ok, current = pcall(function()
		return game:GetService(parts[1])
	end)
	if not ok or not current then
		-- Fallback: try FindFirstChild for non-service top-level instances
		current = game:FindFirstChild(parts[1])
		if not current then
			return nil
		end
	end

	-- Walk remaining parts
	for i = 2, #parts do
		local child = current:FindFirstChild(parts[i])
		if not child then
			return nil
		end
		current = child
	end

	return current
end

local function resolveParentAndName(path: string): (Instance?, string?)
	local parts = string.split(path, ".")
	if #parts < 2 then
		return nil, nil
	end

	local parentPath = table.concat(parts, ".", 1, #parts - 1)
	local name = parts[#parts]
	local parent = resolvePath(parentPath)

	return parent, name
end

return {
	resolvePath = resolvePath,
	resolveParentAndName = resolveParentAndName,
}
