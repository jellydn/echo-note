import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsView } from "../SettingsView";

const mockAudioDevices = [
	{ id: "device1", name: "Built-in Microphone" },
	{ id: "device2", name: "USB Microphone" },
];

const mockWhisperModels = [
	{
		size: "tiny",
		filename: "ggml-tiny.bin",
		expected_size: 75000000,
		is_downloaded: true,
		actual_size: 75000000,
	},
	{
		size: "small",
		filename: "ggml-small.bin",
		expected_size: 466000000,
		is_downloaded: false,
		actual_size: null,
	},
];

// Helper to create mock invoke implementations with customizable overrides
function createMockInvoke(
	overrides: {
		whisperModels?: typeof mockWhisperModels;
		blackholeInstalled?: boolean;
		homebrewInstalled?: boolean;
		llmProvider?: string | null;
		settingValues?: Record<string, string>;
		setSettingReturns?: boolean;
	} = {},
) {
	return async (command: string, args?: unknown) => {
		const whisperModels = overrides.whisperModels ?? mockWhisperModels;
		const blackholeInstalled = overrides.blackholeInstalled ?? true;
		const homebrewInstalled = overrides.homebrewInstalled ?? true;
		const llmProvider = overrides.llmProvider ?? null;
		const settingValues = overrides.settingValues ?? {};
		const setSettingReturns = overrides.setSettingReturns ?? true;

		switch (command) {
			case "list_audio_devices_command":
				return { success: true, data: mockAudioDevices, error: null };
			case "list_whisper_models_command":
				return { success: true, data: whisperModels, error: null };
			case "get_setting_command": {
				const req = args as { request: { key: string } } | undefined;
				const key = req?.request?.key;
				if (key && settingValues[key] !== undefined) {
					return { success: true, data: settingValues[key], error: null };
				}
				if (key === "llm_provider" && llmProvider !== null) {
					return { success: true, data: llmProvider, error: null };
				}
				return { success: true, data: "", error: null };
			}
			case "set_setting_command":
				return { success: true, data: setSettingReturns, error: null };
			case "check_ollama_status_command":
				return {
					success: true,
					data: { available: true, url: "http://localhost:11434" },
					error: null,
				};
			case "check_blackhole_status_command":
				return {
					success: true,
					data: {
						installed: blackholeInstalled,
						device_name: blackholeInstalled ? "BlackHole 2ch" : null,
					},
					error: null,
				};
			case "check_homebrew_status_command":
				return { success: true, data: homebrewInstalled, error: null };
			default:
				return { success: false, data: null, error: "Unknown command" };
		}
	};
}

describe("SettingsView", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("shows loading state initially", () => {
		vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

		render(<SettingsView />);

		expect(screen.getByText("Loading settings...")).toBeInTheDocument();
	});

	it("displays settings after loading", async () => {
		vi.mocked(invoke).mockImplementation(createMockInvoke());

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText("Settings")).toBeInTheDocument();
		});

		expect(screen.getByText("Audio")).toBeInTheDocument();
		expect(screen.getByText("Transcription Model")).toBeInTheDocument();
		expect(screen.getByText("Summary Generation")).toBeInTheDocument();
	});

	it("shows BlackHole installed status", async () => {
		vi.mocked(invoke).mockImplementation(createMockInvoke({ blackholeInstalled: true }));

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/BlackHole installed/i)).toBeInTheDocument();
		});
	});

	it("shows BlackHole not installed warning", async () => {
		vi.mocked(invoke).mockImplementation(
			createMockInvoke({ blackholeInstalled: false, homebrewInstalled: false }),
		);

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/BlackHole not installed/i)).toBeInTheDocument();
		});

		expect(screen.getByRole("button", { name: /download installer/i })).toBeInTheDocument();
	});

	it("shows downloaded and not downloaded model statuses", async () => {
		vi.mocked(invoke).mockImplementation(createMockInvoke());

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText("tiny")).toBeInTheDocument();
		});

		expect(screen.getByText(/✓ Downloaded/)).toBeInTheDocument();
		expect(screen.getAllByRole("button", { name: /download/i }).length).toBeGreaterThan(0);
	});

	it("saves setting when audio device is changed", async () => {
		vi.mocked(invoke).mockImplementation(
			createMockInvoke({ settingValues: { audio_device: "device1" } }),
		);

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByLabelText("Audio Input Device")).toBeInTheDocument();
		});

		const select = screen.getByLabelText("Audio Input Device");
		await userEvent.selectOptions(select, "device2");

		await waitFor(() => {
			expect(screen.getByText("Settings saved successfully")).toBeInTheDocument();
		});
	});

	it("switches to API provider and shows API settings", async () => {
		vi.mocked(invoke).mockImplementation(createMockInvoke({ llmProvider: "api" }));

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByLabelText("API Key")).toBeInTheDocument();
		});

		expect(screen.getByText(/Data will leave your device/i)).toBeInTheDocument();
	});

	it("shows prompt when no models are downloaded", async () => {
		const emptyModels = [
			{
				size: "tiny",
				filename: "ggml-tiny.bin",
				expected_size: 75000000,
				is_downloaded: false,
				actual_size: null,
			},
		];

		vi.mocked(invoke).mockImplementation(createMockInvoke({ whisperModels: emptyModels }));

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/No model downloaded yet/i)).toBeInTheDocument();
		});
	});
});
