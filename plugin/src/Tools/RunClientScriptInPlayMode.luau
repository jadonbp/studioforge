local Main = script:FindFirstAncestor("MCPStudioPlugin")
local ConsoleOutput = require(Main.Utils.ConsoleOutput)
local GlobalVariables = require(Main.Utils.GlobalVariables)
local StartStopPlay = require(Main.Tools.StartStopPlay)
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local ServerScriptService = game:GetService("ServerScriptService")
local StarterPlayer = game:GetService("StarterPlayer")
local StudioTestService = game:GetService("StudioTestService")

local REMOTE_NAME = "MCPClientTestRemote"
local LISTENER_NAME = "MCPClientTestListener"
local CLIENT_SCRIPT_NAME = "MCPClientTestScript"

-- Server-side listener script: receives results from client via RemoteEvent, calls EndTest
local function buildListenerSource(): string
	return [[
local StudioTestService = game:GetService("StudioTestService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local RunService = game:GetService("RunService")

if not RunService:IsRunning() then
	return
end

local remote = ReplicatedStorage:WaitForChild("]] .. REMOTE_NAME .. [[", 30)
if not remote then
	StudioTestService:EndTest({
		success = false,
		error = "MCPClientTestRemote not found",
		logs = {},
		errors = {},
		duration = 0,
		isTimeout = true,
	})
	return
end

remote.OnServerEvent:Connect(function(_player, result)
	StudioTestService:EndTest(result)
end)
]]
end

-- Client-side script: runs user code with output capture, fires result back via RemoteEvent
local function buildClientSource(userCode: string, timeout: number): string
	local timeoutStr = tostring(timeout)
	return [[
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local LogService = game:GetService("LogService")
local RunService = game:GetService("RunService")

if not RunService:IsRunning() then
	return
end

local remote = ReplicatedStorage:WaitForChild("]] .. REMOTE_NAME .. [[", 30)
if not remote then
	warn("[MCP] MCPClientTestRemote not found on client")
	return
end

local capturedLogs = {}
local logConnection = nil

local MESSAGE_TYPE_MAP = {
	[Enum.MessageType.MessageOutput] = "output",
	[Enum.MessageType.MessageInfo] = "info",
	[Enum.MessageType.MessageWarning] = "warning",
	[Enum.MessageType.MessageError] = "error",
}

logConnection = LogService.MessageOut:Connect(function(message, messageType)
	if string.sub(message, 1, 5) == "[MCP]" then return end
	table.insert(capturedLogs, {
		level = MESSAGE_TYPE_MAP[messageType] or "output",
		message = message,
		ts = os.clock(),
	})
end)

local startTime = os.clock()
local isTimeout = false
local hasFired = false

local function sendResult()
	if hasFired then return end
	hasFired = true

	local duration = os.clock() - startTime
	task.wait(0.1)

	if logConnection then logConnection:Disconnect() end

	local errors = {}
	for _, log in ipairs(capturedLogs) do
		if log.level == "error" or log.level == "warning" then
			table.insert(errors, log)
		end
	end

	remote:FireServer({
		success = ok,
		value = if ok then tostring(result) else nil,
		error = if not ok then tostring(result) else nil,
		logs = capturedLogs,
		errors = errors,
		duration = duration,
		isTimeout = isTimeout,
	})
end

task.spawn(function()
	wait(]] .. timeoutStr .. [[)
	isTimeout = true
	sendResult()
end)

local ok, result = pcall(function()
]] .. userCode .. [[

end)

sendResult()
]]
end

local function removeInjectedInstances()
	local remote = ReplicatedStorage:FindFirstChild(REMOTE_NAME)
	if remote then remote:Destroy() end

	local listener = ServerScriptService:FindFirstChild(LISTENER_NAME)
	if listener then listener:Destroy() end

	local clientScript = StarterPlayer.StarterPlayerScripts:FindFirstChild(CLIENT_SCRIPT_NAME)
	if clientScript then clientScript:Destroy() end
end

local function injectInstances(args: Types.RunClientScriptInPlayModeArgs)
	removeInjectedInstances()

	-- 1. Create RemoteEvent
	local remote = Instance.new("RemoteEvent")
	remote.Name = REMOTE_NAME
	remote.Parent = ReplicatedStorage

	-- 2. Create server listener script
	local listener = Instance.new("Script")
	listener.Name = LISTENER_NAME
	listener.Source = buildListenerSource()
	listener.Parent = ServerScriptService

	-- 3. Create client test script
	local clientScript = Instance.new("LocalScript")
	clientScript.Name = CLIENT_SCRIPT_NAME
	clientScript.Source = buildClientSource(args.code, args.timeout or 30)
	clientScript.Parent = StarterPlayer.StarterPlayerScripts
end

local function runClientScriptInPlayMode(args: Types.RunClientScriptInPlayModeArgs): string?
	injectInstances(args)

	ConsoleOutput.outputMessage = ""
	local success, result = pcall(function()
		GlobalVariables.studioMode = "start_play"
		return StudioTestService:ExecutePlayModeAsync({})
	end)

	GlobalVariables.studioMode = "stop"
	removeInjectedInstances()

	if not success then
		error("Failed to run client script in play mode: " .. result)
	end

	local resultStr = HttpService:JSONEncode(result)
	return resultStr
end

local function handleRunClientScriptInPlayMode(args: Types.RunClientScriptInPlayModeArgs): string?
	if type(args.code) ~= "string" or args.code == "" then
		error("Missing code in RunClientScriptInPlayMode")
	end

	if args.timeout ~= nil then
		if type(args.timeout) ~= "number" or args.timeout < 0 then
			error("Timeout must be a number greater than 0 in RunClientScriptInPlayMode")
		end
	end

	StartStopPlay({ mode = "stop" })
	wait(0.2)

	return runClientScriptInPlayMode(args)
end

return handleRunClientScriptInPlayMode :: Types.ToolFunction
