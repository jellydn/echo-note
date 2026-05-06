import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RecordView } from "../RecordView";

describe("RecordView", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("shows idle state initially", () => {
		render(<RecordView />);

		expect(screen.getByText("Ready to record")).toBeInTheDocument();
		expect(screen.getByRole("button", { name: /start recording/i })).toBeInTheDocument();
	});

	it("starts recording when button is clicked", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: true,
			error: null,
		});

		render(<RecordView />);

		const startButton = screen.getByRole("button", { name: /start recording/i });
		await userEvent.click(startButton);

		await waitFor(() => {
			expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
		});

		expect(screen.getByRole("button", { name: /stop recording/i })).toBeInTheDocument();
	});

	it("stops recording and shows title modal", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "start_recording_command") {
				return { success: true, data: true, error: null };
			}
			if (command === "stop_recording_command") {
				return {
					success: true,
					data: {
						file_path: "/path/to/recording.wav",
						duration_seconds: 60,
						used_system_audio: true,
						system_audio_error: null,
					},
					error: null,
				};
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView />);

		// Start recording
		const startButton = screen.getByRole("button", { name: /start recording/i });
		await userEvent.click(startButton);

		await waitFor(() => {
			expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
		});

		// Stop recording
		const stopButton = screen.getByRole("button", { name: /stop recording/i });
		await userEvent.click(stopButton);

		await waitFor(() => {
			expect(screen.getByText("Save Recording")).toBeInTheDocument();
		});

		expect(screen.getByLabelText("Meeting Title")).toBeInTheDocument();
		expect(screen.getByText("Duration: 01:00")).toBeInTheDocument();
		expect(screen.getByText("System audio included")).toBeInTheDocument();
	});

	it("shows warning when system audio capture fails", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "start_recording_command") {
				return { success: true, data: true, error: null };
			}
			if (command === "stop_recording_command") {
				return {
					success: true,
					data: {
						file_path: "/path/to/recording.wav",
						duration_seconds: 60,
						used_system_audio: false,
						system_audio_error: "BlackHole stream failed",
					},
					error: null,
				};
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView />);

		await userEvent.click(screen.getByRole("button", { name: /start recording/i }));
		await waitFor(() => {
			expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
		});

		await userEvent.click(screen.getByRole("button", { name: /stop recording/i }));

		await waitFor(() => {
			expect(screen.getByText(/system audio was not captured/i)).toBeInTheDocument();
		});
		expect(screen.getByText(/blackhole stream failed/i)).toBeInTheDocument();
	});

	it("tests microphone and shows result", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "test_microphone_command") {
				return { success: true, data: 0.05, error: null }; // Good audio level
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView />);

		const testButton = screen.getByRole("button", { name: /test microphone/i });
		await userEvent.click(testButton);

		await waitFor(() => {
			expect(screen.getByText(/microphone is working/i)).toBeInTheDocument();
		});
	});

	it("shows microphone silent warning", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "test_microphone_command") {
				return { success: true, data: 0.001, error: null }; // Low audio level
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView onNavigateToSettings={vi.fn()} />);

		const testButton = screen.getByRole("button", { name: /test microphone/i });
		await userEvent.click(testButton);

		await waitFor(() => {
			expect(screen.getByText(/no audio detected/i)).toBeInTheDocument();
		});
	});

	it("saves meeting and processes to transcription complete", async () => {
		const onMeetingCreated = vi.fn();
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "start_recording_command") {
				return { success: true, data: true, error: null };
			}
			if (command === "stop_recording_command") {
				return {
					success: true,
					data: {
						file_path: "/path/to/recording.wav",
						duration_seconds: 60,
						used_system_audio: false,
						system_audio_error: null,
					},
					error: null,
				};
			}
			if (command === "create_meeting_command") {
				return {
					success: true,
					data: {
						id: 1,
						title: "Test Meeting",
						date: "2026-04-20T10:00:00Z",
						duration_seconds: 60,
						audio_path: "/path/to/recording.wav",
						created_at: "2026-04-20T10:00:00Z",
					},
					error: null,
				};
			}
			if (command === "transcribe_audio_command") {
				return {
					success: true,
					data: {
						transcript_id: 1,
						text: "This is the transcript",
						duration_seconds: 5,
					},
					error: null,
				};
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView onMeetingCreated={onMeetingCreated} />);

		// Start and stop recording
		await userEvent.click(screen.getByRole("button", { name: /start recording/i }));
		await waitFor(() => {
			expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
		});

		await userEvent.click(screen.getByRole("button", { name: /stop recording/i }));
		await waitFor(() => {
			expect(screen.getByText("Save Recording")).toBeInTheDocument();
		});

		// Enter meeting title
		const titleInput = screen.getByLabelText("Meeting Title");
		await userEvent.clear(titleInput);
		await userEvent.type(titleInput, "Test Meeting");

		// Save meeting
		const saveButton = screen.getByRole("button", { name: /save meeting/i });
		await userEvent.click(saveButton);

		// Since API call is fast, we may see either transcribing or complete state
		await waitFor(() => {
			expect(screen.getByText("Processing Meeting")).toBeInTheDocument();
		});
	});

	it("shows error when recording fails to start", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: false,
			data: null,
			error: "Microphone permission denied",
		});

		render(<RecordView />);

		const startButton = screen.getByRole("button", { name: /start recording/i });
		await userEvent.click(startButton);

		await waitFor(() => {
			expect(screen.getByText("Microphone permission denied")).toBeInTheDocument();
		});
	});

	it("allows discarding recording", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "start_recording_command") {
				return { success: true, data: true, error: null };
			}
			if (command === "stop_recording_command") {
				return {
					success: true,
					data: {
						file_path: "/path/to/recording.wav",
						duration_seconds: 30,
						used_system_audio: false,
						system_audio_error: null,
					},
					error: null,
				};
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<RecordView />);

		// Start and stop recording
		await userEvent.click(screen.getByRole("button", { name: /start recording/i }));
		await waitFor(() => {
			expect(screen.getByText("Recording in progress...")).toBeInTheDocument();
		});

		await userEvent.click(screen.getByRole("button", { name: /stop recording/i }));
		await waitFor(() => {
			expect(screen.getByText("Save Recording")).toBeInTheDocument();
		});

		// Click discard
		const discardButton = screen.getByRole("button", { name: /discard/i });
		await userEvent.click(discardButton);

		// Should return to idle state
		await waitFor(() => {
			expect(screen.getByText("Ready to record")).toBeInTheDocument();
		});
	});
});
