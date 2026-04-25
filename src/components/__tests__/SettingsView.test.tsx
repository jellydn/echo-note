import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsView } from "../SettingsView";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
	invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: vi.fn().mockResolvedValue(() => {}),
}));

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
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command":
					return { success: true, data: "", error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText("Settings")).toBeInTheDocument();
		});

		expect(screen.getByText("Audio")).toBeInTheDocument();
		expect(screen.getByText("Transcription Model")).toBeInTheDocument();
		expect(screen.getByText("Summary Generation")).toBeInTheDocument();
	});

	it("shows BlackHole installed status", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command":
					return { success: true, data: "", error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/BlackHole installed/i)).toBeInTheDocument();
		});
	});

	it("shows BlackHole not installed warning", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command":
					return { success: true, data: "", error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return { success: true, data: { installed: false, device_name: null }, error: null };
				case "check_homebrew_status_command":
					return { success: true, data: false, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/BlackHole not installed/i)).toBeInTheDocument();
		});

		expect(screen.getByRole("button", { name: /download installer/i })).toBeInTheDocument();
	});

	it("shows downloaded and not downloaded model statuses", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command":
					return { success: true, data: "", error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText("tiny")).toBeInTheDocument();
		});

		expect(screen.getByText(/✓ Downloaded/)).toBeInTheDocument();
		expect(screen.getByRole("button", { name: /download/i })).toBeInTheDocument();
	});

	it("saves setting when audio device is changed", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command": {
					const req = args as { request: { key: string } };
					if (req?.request?.key === "audio_device") {
						return { success: true, data: "device1", error: null };
					}
					return { success: true, data: "", error: null };
				}
				case "set_setting_command":
					return { success: true, data: true, error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

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
		vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: mockWhisperModels, error: null };
				case "get_setting_command": {
					const req = args as { request: { key: string } };
					if (req?.request?.key === "llm_provider") {
						return { success: true, data: "api", error: null };
					}
					return { success: true, data: "", error: null };
				}
				case "set_setting_command":
					return { success: true, data: true, error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

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

		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "list_audio_devices_command":
					return { success: true, data: mockAudioDevices, error: null };
				case "list_whisper_models_command":
					return { success: true, data: emptyModels, error: null };
				case "get_setting_command":
					return { success: true, data: "", error: null };
				case "check_ollama_status_command":
					return {
						success: true,
						data: { available: true, url: "http://localhost:11434" },
						error: null,
					};
				case "check_blackhole_status_command":
					return {
						success: true,
						data: { installed: true, device_name: "BlackHole 2ch" },
						error: null,
					};
				case "check_homebrew_status_command":
					return { success: true, data: true, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<SettingsView />);

		await waitFor(() => {
			expect(screen.getByText(/No model downloaded yet/i)).toBeInTheDocument();
		});
	});
});
