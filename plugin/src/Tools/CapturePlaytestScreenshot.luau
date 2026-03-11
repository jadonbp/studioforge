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

-- Wait for the character to load and the world to render
local character = player.Character or player.CharacterAdded:Wait()
character:WaitForChild("HumanoidRootPart", 10)
task.wait(2)

local captureComplete = false
local captureContentId = nil

CaptureService:CaptureScreenshot(function(contentId)
	captureContentId = contentId
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

if captureComplete and captureContentId then
	print("[MCP_SCREENSHOT]" .. tostring(captureContentId))
else
	print("[MCP_SCREENSHOT]ERROR:Screenshot capture timed out or failed")
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

	-- Parse the result to extract the contentId from logs
	local parsed = HttpService:JSONDecode(result)
	if parsed and parsed.logs then
		for _, log in ipairs(parsed.logs) do
			if type(log.message) == "string" and string.sub(log.message, 1, 16) == "[MCP_SCREENSHOT]" then
				local data = string.sub(log.message, 17)

				-- Check for error
				if string.sub(data, 1, 6) == "ERROR:" then
					error("Screenshot capture failed: " .. string.sub(data, 7))
				end

				-- Return the contentId with the __screenshot__ prefix
				-- The Rust side will resolve the contentId to a file path and read it
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
