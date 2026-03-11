local Main = script:FindFirstAncestor("MCPStudioPlugin")
local RunClientScriptInPlayMode = require(Main.Tools.RunClientScriptInPlayMode)
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")

local function buildScreenshotScript(includeUI: boolean): string
	local captureMode = if includeUI then "Enum.UICaptureMode.IncludeUI" else "Enum.UICaptureMode.None"
	return [[
local CaptureService = game:GetService("CaptureService")
local Players = game:GetService("Players")

local player = Players.LocalPlayer
if not player then
	player = Players.PlayerAdded:Wait()
end

-- Wait for the game to render a few frames
task.wait(1)

-- Base64 encoding table
local b64chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

local function base64encode(data: buffer): string
	local len = buffer.len(data)
	local result = {}
	local i = 0

	while i < len do
		local b0 = buffer.readu8(data, i)
		local b1 = if i + 1 < len then buffer.readu8(data, i + 1) else 0
		local b2 = if i + 2 < len then buffer.readu8(data, i + 2) else 0

		local n = b0 * 65536 + b1 * 256 + b2

		table.insert(result, string.sub(b64chars, bit32.rshift(n, 18) + 1, bit32.rshift(n, 18) + 1))
		table.insert(result, string.sub(b64chars, bit32.band(bit32.rshift(n, 12), 63) + 1, bit32.band(bit32.rshift(n, 12), 63) + 1))

		if i + 1 < len then
			table.insert(result, string.sub(b64chars, bit32.band(bit32.rshift(n, 6), 63) + 1, bit32.band(bit32.rshift(n, 6), 63) + 1))
		else
			table.insert(result, "=")
		end

		if i + 2 < len then
			table.insert(result, string.sub(b64chars, bit32.band(n, 63) + 1, bit32.band(n, 63) + 1))
		else
			table.insert(result, "=")
		end

		i = i + 3
	end

	return table.concat(result)
end

local captureComplete = false
local captureResult = nil

CaptureService:CaptureScreenshot(function(contentId)
	-- Try to extract image data using EditableImage
	local success, err = pcall(function()
		local editableImage = game:GetService("AssetService"):CreateEditableImageAsync(contentId)
		if not editableImage then
			error("Failed to create EditableImage from capture")
		end

		local width = editableImage.Size.X
		local height = editableImage.Size.Y

		-- Read all pixels as a buffer (RGBA format)
		local pixelBuffer = editableImage:ReadPixelsBuffer(Vector2.zero, editableImage.Size)

		-- We need to encode this as PNG. Since encoding PNG in Luau is complex,
		-- we'll send the raw RGBA pixel data with dimensions as a simpler approach.
		-- The MCP server can convert it if needed, or we use a simpler format.

		-- For now, encode the raw pixel buffer as base64 with a header containing dimensions
		local headerStr = width .. "x" .. height .. ":"
		local b64Data = base64encode(pixelBuffer)

		captureResult = headerStr .. b64Data
		editableImage:Destroy()
	end)

	if not success then
		captureResult = "ERROR:" .. tostring(err)
	end

	captureComplete = true
end, {
	UICaptureMode = ]] .. captureMode .. [[,
})

-- Wait for capture to complete (max 10 seconds)
local waitTime = 0
while not captureComplete and waitTime < 10 do
	task.wait(0.1)
	waitTime = waitTime + 0.1
end

if not captureComplete then
	print("[MCP_SCREENSHOT]ERROR:Screenshot capture timed out")
elseif captureResult then
	-- Split into chunks to avoid log message size limits
	local prefix = "[MCP_SCREENSHOT]"
	print(prefix .. captureResult)
else
	print("[MCP_SCREENSHOT]ERROR:No capture result")
end
]]
end

local function handleCapturePlaytestScreenshot(args: Types.CapturePlaytestScreenshotArgs): string?
	local includeUI = true
	if args.includeUI == false then
		includeUI = false
	end

	local screenshotCode = buildScreenshotScript(includeUI)

	-- Run the screenshot capture as a client script
	local result = RunClientScriptInPlayMode({
		code = screenshotCode,
		timeout = 20,
	})

	if not result then
		error("No result from client script execution")
	end

	-- Parse the result to extract screenshot data from logs
	local parsed = HttpService:JSONDecode(result)
	if parsed and parsed.logs then
		for _, log in ipairs(parsed.logs) do
			if type(log.message) == "string" and string.sub(log.message, 1, 16) == "[MCP_SCREENSHOT]" then
				local data = string.sub(log.message, 17)

				-- Check for error
				if string.sub(data, 1, 6) == "ERROR:" then
					error("Screenshot capture failed: " .. string.sub(data, 7))
				end

				-- Parse dimensions and base64 data: "WIDTHxHEIGHT:BASE64DATA"
				-- For now, return the raw RGBA base64 with the __screenshot__ prefix
				-- The Rust side will handle this as image content
				return "__screenshot__:" .. data
			end
		end
	end

	-- If no screenshot data found, return info about what happened
	if parsed and parsed.errors and #parsed.errors > 0 then
		local errorMessages = {}
		for _, err in ipairs(parsed.errors) do
			table.insert(errorMessages, err.message)
		end
		error("Screenshot capture failed with errors: " .. table.concat(errorMessages, "; "))
	end

	error("Screenshot capture did not produce any output. CaptureService may not be available in Studio playtest context.")
end

return handleCapturePlaytestScreenshot :: Types.ToolFunction
