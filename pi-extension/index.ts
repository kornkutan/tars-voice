import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { execFile } from "node:child_process";
import { readFileSync } from "node:fs";
import * as path from "node:path";
import * as os from "node:os";

interface VoiceConfig {
	binaryPath?: string;
	statePath?: string;
	pollIntervalMs?: number;
}

const CONFIG_PATH = path.join(
	path.dirname(new URL(import.meta.url).pathname),
	"config.json",
);

function loadConfig(): VoiceConfig {
	try {
		return JSON.parse(readFileSync(CONFIG_PATH, "utf-8"));
	} catch {
		return {};
	}
}

const DEFAULT_BINARY = path.join(os.homedir(), "bin", "tars-voice");
const DEFAULT_STATE = path.join(
	os.homedir(),
	".pi-agent",
	"tars-voice",
	"state.json",
);

interface DaemonState {
	state: string;
	transcript: string;
	response: string;
	updated_at: number;
	pid: number;
	cwd: string;
	session_id: string;
	error: string;
}

function readState(statePath: string): DaemonState | null {
	try {
		return JSON.parse(readFileSync(statePath, "utf-8"));
	} catch {
		return null;
	}
}

function pidAlive(pid: number): boolean {
	if (!pid) return false;
	try {
		process.kill(pid, 0);
		return true;
	} catch {
		return false;
	}
}

function statusText(st: DaemonState | null): string | undefined {
	if (!st) return undefined;
	// treat a stale pid as not running
	if (!pidAlive(st.pid)) return undefined;
	switch (st.state) {
		case "recording":
			return "VOICE: REC";
		case "transcribing":
			return "VOICE: transcribing";
		case "working":
			return st.transcript
				? `VOICE: working (${truncate(st.transcript, 30)})`
				: "VOICE: working";
		case "error":
			return "VOICE: error";
		case "starting":
			return "VOICE: starting";
		case "idle":
		default:
			return "VOICE: idle";
	}
}

function truncate(s: string, n: number): string {
	const one = s.replace(/\s+/g, " ").trim();
	return one.length <= n ? one : one.slice(0, n - 1) + "…";
}

function runBinary(
	binary: string,
	args: string[],
	cwd: string,
): Promise<{ code: number; stdout: string; stderr: string }> {
	return new Promise((resolve) => {
		execFile(
			binary,
			args,
			{ cwd, timeout: 20_000 },
			(err, stdout, stderr) => {
				resolve({
					code: err && "code" in err ? (err.code as number) : err ? 1 : 0,
					stdout: String(stdout).trim(),
					stderr: String(stderr).trim(),
				});
			},
		);
	});
}

export default function (pi: ExtensionAPI) {
	let pollTimer: ReturnType<typeof setInterval> | null = null;

	pi.on("session_start", async (_event, ctx) => {
		if (ctx.mode !== "tui") return;
		const cfg = loadConfig();
		const binary = cfg.binaryPath ?? DEFAULT_BINARY;
		const statePath = cfg.statePath ?? DEFAULT_STATE;
		const intervalMs = cfg.pollIntervalMs ?? 1000;

		const refresh = () => {
			const text = statusText(readState(statePath));
			try {
				ctx.ui.setStatus("voice", text);
			} catch {
				// setStatus may be unavailable in some modes; ignore
			}
		};

		refresh();
		pollTimer = setInterval(refresh, intervalMs);
	});

	pi.on("session_shutdown", () => {
		if (pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
	});

	pi.registerCommand("voice", {
		description:
			"Voice control: /voice start|stop|status (push-to-talk via tars-voice daemon)",
		handler: async (args, ctx) => {
			const cfg = loadConfig();
			const binary = cfg.binaryPath ?? DEFAULT_BINARY;
			const cwd = ctx.sessionManager.getCwd();
			const sub = (args ?? "").trim().split(/\s+/)[0] ?? "status";

			if (sub === "start") {
				const res = await runBinary(binary, ["start", cwd], cwd);
				if (res.code === 0) {
					ctx.ui.notify(res.stdout || "tars-voice started", "info");
				} else {
					ctx.ui.notify(
						res.stderr || res.stdout || "failed to start tars-voice",
						"error",
					);
				}
			} else if (sub === "stop") {
				const res = await runBinary(binary, ["stop"], cwd);
				ctx.ui.notify(res.stdout || "stopped", res.code === 0 ? "info" : "warning");
			} else {
				const res = await runBinary(binary, ["status"], cwd);
				const msg = res.stdout || res.stderr || "no status";
				ctx.ui.notify(msg, res.code === 0 ? "info" : "warning");
			}
		},
	});
}