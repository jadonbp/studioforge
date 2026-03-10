export type InsertModelArgs = {
	query: string,
}

export type RunCodeArgs = {
	command: string,
}

export type TestMode = "start_play" | "run_server"

export type StartStopPlayMode = TestMode | "stop"

export type StartStopPlayArgs = {
	mode: StartStopPlayMode,
}

export type RunScriptInPlayModeArgs = {
	code: string,
	timeout: number?,
	mode: TestMode,
}

-- StudioForge new tool argument types

export type ReadScriptArgs = {
	path: string,
}

export type WriteScriptArgs = {
	path: string,
	source: string,
	scriptType: string?,
}

export type GetChildrenArgs = {
	path: string,
}

export type GetPropertiesArgs = {
	path: string,
}

export type GetSelectionArgs = {}

export type ToolArgs =
	{}
	| { InsertModel: InsertModelArgs }
	| { RunCode: RunCodeArgs }
	| { StartStopPlay: StartStopPlayArgs }
	| { RunScriptInPlayMode: RunScriptInPlayModeArgs }
	| { ReadScript: ReadScriptArgs }
	| { WriteScript: WriteScriptArgs }
	| { GetChildren: GetChildrenArgs }
	| { GetProperties: GetPropertiesArgs }
	| { GetSelection: GetSelectionArgs }

export type ToolFunction = (ToolArgs) -> string?

return {}
