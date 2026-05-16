import { invoke } from "@tauri-apps/api/core";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HistoryView } from "../HistoryView";

const mockMeetings = [
	{
		id: 1,
		title: "Team Standup",
		date: "2026-04-20T10:00:00Z",
		duration_seconds: 900,
		audio_path: "/path/to/audio1.wav",
		created_at: "2026-04-20T10:00:00Z",
	},
	{
		id: 2,
		title: "Product Review",
		date: "2026-04-19T14:30:00Z",
		duration_seconds: 1800,
		audio_path: "/path/to/audio2.wav",
		created_at: "2026-04-19T14:30:00Z",
	},
];

describe("HistoryView", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("shows loading state initially", () => {
		vi.mocked(invoke).mockImplementation(() => new Promise(() => {}));

		render(<HistoryView />);

		expect(screen.getByText("Loading meetings...")).toBeInTheDocument();
	});

	it("displays meetings list after loading", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: mockMeetings,
			error: null,
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});

		expect(screen.getByText("Product Review")).toBeInTheDocument();
		expect(screen.getByText("2 meetings recorded")).toBeInTheDocument();
	});

	it("shows empty state when no meetings exist", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: [],
			error: null,
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("No meetings yet")).toBeInTheDocument();
		});

		expect(screen.getByText("Record your first meeting to see it here.")).toBeInTheDocument();
	});

	it("shows error state when API fails", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: false,
			data: null,
			error: "Failed to connect to database",
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("Failed to connect to database")).toBeInTheDocument();
		});

		expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
	});

	it("calls onMeetingClick when meeting is clicked", async () => {
		const onMeetingClick = vi.fn();
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: mockMeetings,
			error: null,
		});

		render(<HistoryView onMeetingClick={onMeetingClick} />);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});

		// Find the meeting item by role and name for resilience
		const meetingItem = screen.getAllByRole("button", { name: /team standup/i })[0];
		await userEvent.click(meetingItem);

		expect(onMeetingClick).toHaveBeenCalledWith(1);
	});

	it("shows delete confirmation modal when delete is clicked", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: mockMeetings,
			error: null,
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});

		const deleteButton = screen.getByLabelText("Delete Team Standup");
		await userEvent.click(deleteButton);

		expect(screen.getByText("Delete Meeting?")).toBeInTheDocument();
		expect(screen.getByText(/Are you sure you want to delete this meeting/i)).toBeInTheDocument();
	});

	it("cancels delete when cancel button is clicked", async () => {
		vi.mocked(invoke).mockResolvedValue({
			success: true,
			data: mockMeetings,
			error: null,
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});

		const deleteButton = screen.getByLabelText("Delete Team Standup");
		await userEvent.click(deleteButton);

		const cancelButton = screen.getByRole("button", { name: /cancel/i });
		await userEvent.click(cancelButton);

		await waitFor(() => {
			expect(screen.queryByText("Delete Meeting?")).not.toBeInTheDocument();
		});

		// Meeting should still be visible
		expect(screen.getByText("Team Standup")).toBeInTheDocument();
	});

	it("deletes meeting when confirm is clicked", async () => {
		const onDeleteMeeting = vi.fn();
		vi.mocked(invoke).mockImplementation(async (command: string) => {
			if (command === "list_meetings_command") {
				return { success: true, data: mockMeetings, error: null };
			}
			if (command === "delete_meeting_command") {
				return { success: true, data: true, error: null };
			}
			return { success: false, data: null, error: "Unknown command" };
		});

		render(<HistoryView onDeleteMeeting={onDeleteMeeting} />);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});

		const deleteButton = screen.getByLabelText("Delete Team Standup");
		await userEvent.click(deleteButton);

		// Look for the danger/delete button in the modal (more specific selector)
		const confirmButton = screen.getByRole("button", { name: /^delete$/i });
		await userEvent.click(confirmButton);

		await waitFor(() => {
			expect(onDeleteMeeting).toHaveBeenCalledWith(1);
		});
	});

	it("retries loading when retry button is clicked", async () => {
		vi.mocked(invoke).mockResolvedValueOnce({
			success: false,
			data: null,
			error: "Network error",
		});

		render(<HistoryView />);

		await waitFor(() => {
			expect(screen.getByText("Network error")).toBeInTheDocument();
		});

		// Setup success for retry
		vi.mocked(invoke).mockResolvedValueOnce({
			success: true,
			data: mockMeetings,
			error: null,
		});

		const retryButton = screen.getByRole("button", { name: /retry/i });
		await userEvent.click(retryButton);

		await waitFor(() => {
			expect(screen.getByText("Team Standup")).toBeInTheDocument();
		});
	});
});
