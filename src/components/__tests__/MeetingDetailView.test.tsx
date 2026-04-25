import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MeetingDetailView } from "../MeetingDetailView";

const mockMeeting = {
	id: 1,
	title: "Product Planning Meeting",
	date: "2026-04-20T10:00:00Z",
	duration_seconds: 1800,
	audio_path: "/path/to/audio.wav",
	created_at: "2026-04-20T10:00:00Z",
};

const mockTranscript = {
	id: 1,
	meeting_id: 1,
	content: "We discussed the roadmap for Q2 and decided to prioritize the mobile app.",
	created_at: "2026-04-20T10:05:00Z",
};

const mockSummary = {
	id: 1,
	meeting_id: 1,
	key_points: "- Discussed Q2 roadmap\n- Mobile app is top priority\n- Need more resources",
	decisions: "- Approved Q2 plan\n- Hired 2 more developers",
	action_items: "- John: Prepare design mockups\n- Sarah: Schedule follow-up",
	created_at: "2026-04-20T10:10:00Z",
};

describe("MeetingDetailView", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("shows loading state initially", () => {
		vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

		render(<MeetingDetailView meetingId={1} />);

		expect(screen.getByText("Loading meeting details...")).toBeInTheDocument();
	});

	it("displays meeting details with transcript and summary", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: mockTranscript, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: mockSummary, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		// Check for duration format (30:00 for 1800 seconds)
		expect(screen.getByText(/30:00/)).toBeInTheDocument();
		expect(screen.getByText("Summary")).toBeInTheDocument();
		expect(screen.getByText("Transcript")).toBeInTheDocument();
	});

	it("shows error when meeting not found", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: false,
			data: null,
			error: "Meeting not found",
		});

		render(<MeetingDetailView meetingId={999} />);

		await waitFor(() => {
			expect(screen.getByText("Meeting not found")).toBeInTheDocument();
		});
	});

	it("calls onBack when back button is clicked", async () => {
		const onBack = vi.fn();
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} onBack={onBack} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		const backButton = screen.getByRole("button", { name: /back/i });
		await userEvent.click(backButton);

		expect(onBack).toHaveBeenCalled();
	});

	it("enters edit mode when title is clicked", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		// Find edit button by title attribute or class
		const editButton = screen.getByTitle("Edit title");
		await userEvent.click(editButton);

		expect(screen.getByRole("textbox")).toBeInTheDocument();
	});

	it("saves edited title when save is clicked", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				case "update_meeting_command":
					return {
						success: true,
						data: { ...mockMeeting, title: "Updated Meeting Title" },
						error: null,
					};
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		// Find edit button by title attribute or class
		const editButton = screen.getByTitle("Edit title");
		await userEvent.click(editButton);

		const input = screen.getByRole("textbox");
		await userEvent.clear(input);
		await userEvent.type(input, "Updated Meeting Title");

		// Save button
		const saveButton = screen.getByRole("button", { name: /save title/i });
		await userEvent.click(saveButton);

		await waitFor(() => {
			expect(screen.getByText("Updated Meeting Title")).toBeInTheDocument();
		});
	});

	it("cancels title edit when cancel is clicked", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		// Find edit button by title attribute or class
		const editButton = screen.getByTitle("Edit title");
		await userEvent.click(editButton);

		const input = screen.getByRole("textbox");
		await userEvent.type(input, " Some Extra Text");

		// Cancel button
		const cancelButton = screen.getByRole("button", { name: /cancel editing/i });
		await userEvent.click(cancelButton);

		// Original title should still be displayed
		expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
	});

	it("copies transcript to clipboard", async () => {
		const mockClipboard = {
			writeText: vi.fn().mockResolvedValue(undefined),
		};
		Object.assign(navigator, { clipboard: mockClipboard });

		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: mockTranscript, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Transcript")).toBeInTheDocument();
		});

		// Find the copy button for transcript
		const copyButton = screen.getByRole("button", { name: /copy transcript/i });
		await userEvent.click(copyButton);

		expect(navigator.clipboard.writeText).toHaveBeenCalledWith(mockTranscript.content);
	});

	it("copies summary to clipboard", async () => {
		const mockClipboard = {
			writeText: vi.fn().mockResolvedValue(undefined),
		};
		Object.assign(navigator, { clipboard: mockClipboard });

		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: mockSummary, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Summary")).toBeInTheDocument();
		});

		// Find the copy button for summary
		const copyButton = screen.getByRole("button", { name: /copy summary/i });
		await userEvent.click(copyButton);

		expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
			expect.stringContaining("Key Points:"),
		);
	});

	it("shows no content message when transcript and summary are missing", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: null, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Product Planning Meeting")).toBeInTheDocument();
		});

		expect(screen.getByText("No transcript available for this meeting.")).toBeInTheDocument();
		expect(screen.getByText("No summary generated for this meeting.")).toBeInTheDocument();
	});

	it("parses and displays key points from summary", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: mockSummary, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Key Points")).toBeInTheDocument();
		});

		expect(screen.getByText("Discussed Q2 roadmap")).toBeInTheDocument();
		expect(screen.getByText("Mobile app is top priority")).toBeInTheDocument();
	});

	it("parses and displays decisions from summary", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: mockSummary, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Decisions")).toBeInTheDocument();
		});

		expect(screen.getByText("Approved Q2 plan")).toBeInTheDocument();
		expect(screen.getByText("Hired 2 more developers")).toBeInTheDocument();
	});

	it("parses and displays action items from summary", async () => {
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			switch (command) {
				case "get_meeting_command":
					return { success: true, data: mockMeeting, error: null };
				case "get_transcript_by_meeting_command":
					return { success: true, data: null, error: null };
				case "get_summary_by_meeting_command":
					return { success: true, data: mockSummary, error: null };
				default:
					return { success: false, data: null, error: "Unknown command" };
			}
		});

		render(<MeetingDetailView meetingId={1} />);

		await waitFor(() => {
			expect(screen.getByText("Action Items")).toBeInTheDocument();
		});

		expect(screen.getByText("John: Prepare design mockups")).toBeInTheDocument();
		expect(screen.getByText("Sarah: Schedule follow-up")).toBeInTheDocument();
	});
});
