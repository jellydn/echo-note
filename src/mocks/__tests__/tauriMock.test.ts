import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installTauriMock, parseMockConfig } from "../tauriMock";

// The mock installs window.__TAURI_INTERNALS__.invoke — the exact surface
// @tauri-apps/api's invoke() routes through in the real app. These tests drive
// it directly with the arg shapes the views actually send, so the arg-contract
// fixes (request-wrapped keys, camelCase args, full install-result shape) are
// locked in and can't silently regress.

const invoke = (cmd: string, args?: unknown): Promise<unknown> => {
	const internals = (
		window as unknown as {
			__TAURI_INTERNALS__?: { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
		}
	).__TAURI_INTERNALS__;
	if (!internals?.invoke) {
		throw new Error("Tauri mock not installed");
	}
	return internals.invoke(cmd, args);
};

interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

describe("parseMockConfig", () => {
	it("defaults to the main shell with BlackHole installed", () => {
		expect(parseMockConfig("")).toEqual({ firstLaunch: false, blackholeInstalled: true });
		expect(parseMockConfig("?other=1")).toEqual({
			firstLaunch: false,
			blackholeInstalled: true,
		});
	});

	it("honors the firstLaunch and blackhole URL flags", () => {
		expect(parseMockConfig("?firstLaunch=1")).toEqual({
			firstLaunch: true,
			blackholeInstalled: true,
		});
		expect(parseMockConfig("?blackhole=0")).toEqual({
			firstLaunch: false,
			blackholeInstalled: false,
		});
		expect(parseMockConfig("?firstLaunch=true&blackhole=false")).toEqual({
			firstLaunch: true,
			blackholeInstalled: false,
		});
	});
});

describe("tauriMock arg contract", () => {
	beforeEach(() => {
		installTauriMock();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("get_setting_command reads the request-wrapped key the view sends", async () => {
		const res = (await invoke("get_setting_command", {
			request: { key: "audio_device" },
		})) as ApiResponse<string>;
		expect(res.success).toBe(true);
		expect(res.data).toBe("MacBook Pro Microphone");
	});

	it("get_setting_command without the request wrapper returns empty (contract)", async () => {
		const res = (await invoke("get_setting_command", {
			key: "audio_device",
		})) as ApiResponse<string>;
		expect(res.data).toBe("");
	});

	it("set_setting_command round-trips through the request wrapper", async () => {
		const setRes = (await invoke("set_setting_command", {
			request: { key: "test_threshold", value: "0.9" },
		})) as ApiResponse<boolean>;
		expect(setRes.success).toBe(true);

		const getRes = (await invoke("get_setting_command", {
			request: { key: "test_threshold" },
		})) as ApiResponse<string>;
		expect(getRes.data).toBe("0.9");
	});

	it("download_whisper_model_command honors the camelCase modelSize arg", async () => {
		vi.useFakeTimers();
		const promise = invoke("download_whisper_model_command", { modelSize: "medium" });
		await vi.advanceTimersByTimeAsync(8 * 350 + 100);
		const res = (await promise) as ApiResponse<string>;
		expect(res.success).toBe(true);
		expect(res.data).toBe("Downloaded medium model (medium)");
	});

	it("download_whisper_model_command with snake_case model_size falls back (contract)", async () => {
		vi.useFakeTimers();
		const promise = invoke("download_whisper_model_command", { model_size: "medium" });
		await vi.advanceTimersByTimeAsync(8 * 350 + 100);
		const res = (await promise) as ApiResponse<string>;
		expect(res.data).toBe("Downloaded small model (small)");
	});

	it("auto_install_blackhole_command returns the full result the wizard checks", async () => {
		const res = (await invoke("auto_install_blackhole_command")) as ApiResponse<{
			success: boolean;
			method: string;
			message: string;
		}>;
		expect(res.success).toBe(true);
		expect(res.data?.success).toBe(true);
		expect(res.data?.method).toBe("bundled");
		expect(typeof res.data?.message).toBe("string");
	});

	it("plugin:opener|open_url opens the url in a new tab", async () => {
		const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
		try {
			await invoke("plugin:opener|open_url", { url: "https://example.com" });
			expect(openSpy).toHaveBeenCalledWith("https://example.com", "_blank", "noopener");
		} finally {
			openSpy.mockRestore();
		}
	});

	it("create → search → delete lifecycle stays consistent", async () => {
		const created = (await invoke("create_meeting_command", {
			request: {
				title: "Contract Lifecycle Test",
				date: "2026-08-03T10:00:00.000Z",
				duration_seconds: 60,
				audio_path: "/tmp/echonote/recordings/contract.wav",
			},
		})) as ApiResponse<{ id: number; title: string }>;
		expect(created.success).toBe(true);
		expect(created.data?.title).toBe("Contract Lifecycle Test");

		const found = (await invoke("search_meetings_command", {
			query: "Contract Lifecycle",
		})) as ApiResponse<Array<{ id: number }>>;
		expect(found.data?.some((m) => m.id === created.data?.id)).toBe(true);

		const deleted = (await invoke("delete_meeting_command", {
			id: created.data?.id,
		})) as ApiResponse<boolean>;
		expect(deleted.success).toBe(true);

		const after = (await invoke("search_meetings_command", {
			query: "Contract Lifecycle",
		})) as ApiResponse<Array<{ id: number }>>;
		expect(after.data?.some((m) => m.id === created.data?.id)).toBe(false);
	});
});

describe("onboarding flow", () => {
	beforeEach(() => {
		installTauriMock();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("complete_first_launch_setup_command flips first-launch off", async () => {
		const before = (await invoke("check_first_launch_status_command")) as ApiResponse<boolean>;
		expect(before.data).toBe(false);

		const res = (await invoke("complete_first_launch_setup_command")) as ApiResponse<boolean>;
		expect(res.success).toBe(true);

		const after = (await invoke("check_first_launch_status_command")) as ApiResponse<boolean>;
		expect(after.data).toBe(false);
	});

	it("blackhole install flow: status → auto-install → status reflects installed", async () => {
		const res = (await invoke("auto_install_blackhole_command")) as ApiResponse<{
			success: boolean;
			method: string;
			message: string;
		}>;
		expect(res.data?.success).toBe(true);
		expect(res.data?.method).toBe("bundled");

		const status = (await invoke("check_blackhole_status_command")) as ApiResponse<{
			installed: boolean;
			device_name: string | null;
		}>;
		expect(status.data?.installed).toBe(true);
		expect(status.data?.device_name).toBe("BlackHole 2ch");
	});
});
