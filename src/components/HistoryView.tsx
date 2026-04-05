import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

// Tauri API response wrapper
interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

// Meeting response from Tauri
interface Meeting {
	id: number;
	title: string;
	date: string;
	duration_seconds: number;
	audio_path: string;
	created_at: string;
}

interface HistoryViewProps {
	onMeetingClick?: (meetingId: number) => void;
	onDeleteMeeting?: (meetingId: number) => void;
}

// Format duration as MM:SS
const formatDuration = (seconds: number): string => {
	const mins = Math.floor(seconds / 60);
	const secs = Math.floor(seconds % 60);
	return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
};

// Format date for display
const formatDate = (dateStr: string): string => {
	const date = new Date(dateStr);
	return date.toLocaleDateString("en-US", {
		month: "short",
		day: "numeric",
		year: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	});
};

export function HistoryView({ onMeetingClick, onDeleteMeeting }: HistoryViewProps) {
	const [meetings, setMeetings] = useState<Meeting[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [deleteConfirmId, setDeleteConfirmId] = useState<number | null>(null);
	const [isDeleting, setIsDeleting] = useState(false);

	// Fetch meetings from the database
	const fetchMeetings = useCallback(async () => {
		setIsLoading(true);
		setError(null);

		try {
			const response = await invoke<ApiResponse<Meeting[]>>("list_meetings_command");

			if (response.success && response.data) {
				// Sort by date (newest first)
				const sortedMeetings = response.data.sort(
					(a, b) => new Date(b.date).getTime() - new Date(a.date).getTime(),
				);
				setMeetings(sortedMeetings);
			} else {
				setError(response.error || "Failed to load meetings");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to load meetings");
		} finally {
			setIsLoading(false);
		}
	}, []);

	// Load meetings on mount
	useEffect(() => {
		fetchMeetings();
	}, [fetchMeetings]);

	// Handle delete confirmation
	const handleDeleteClick = (meetingId: number) => {
		setDeleteConfirmId(meetingId);
	};

	// Confirm delete
	const confirmDelete = async () => {
		if (!deleteConfirmId) return;

		setIsDeleting(true);
		try {
			const response = await invoke<ApiResponse<boolean>>("delete_meeting_command", {
				id: deleteConfirmId,
			});

			if (response.success && response.data) {
				// Remove from local state
				setMeetings((prev) => prev.filter((m) => m.id !== deleteConfirmId));
				if (onDeleteMeeting) {
					onDeleteMeeting(deleteConfirmId);
				}
			} else {
				setError(response.error || "Failed to delete meeting");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to delete meeting");
		} finally {
			setIsDeleting(false);
			setDeleteConfirmId(null);
		}
	};

	// Cancel delete
	const cancelDelete = () => {
		setDeleteConfirmId(null);
	};

	// Handle meeting click
	const handleMeetingClick = (meetingId: number) => {
		if (onMeetingClick) {
			onMeetingClick(meetingId);
		}
	};

	if (isLoading) {
		return (
			<div className="history-view">
				<div className="history-container">
					<div className="history-loading">
						<div className="history-spinner" />
						<p>Loading meetings...</p>
					</div>
				</div>
			</div>
		);
	}

	if (error) {
		return (
			<div className="history-view">
				<div className="history-container">
					<div className="history-error">
						<span className="error-icon">⚠️</span>
						<span>{error}</span>
					</div>
					<button type="button" className="history-retry-button" onClick={fetchMeetings}>
						Retry
					</button>
				</div>
			</div>
		);
	}

	if (meetings.length === 0) {
		return (
			<div className="history-view">
				<div className="history-container">
					<div className="history-empty">
						<div className="history-empty-icon">📋</div>
						<h3>No meetings yet</h3>
						<p>Record your first meeting to see it here.</p>
					</div>
				</div>
			</div>
		);
	}

	return (
		<div className="history-view">
			<div className="history-container">
				<div className="history-header">
					<h2>Meeting History</h2>
					<p className="history-subtitle">
						{meetings.length} {meetings.length === 1 ? "meeting" : "meetings"} recorded
					</p>
				</div>

				<div className="history-list">
					{meetings.map((meeting) => (
						// biome-ignore lint/a11y/useSemanticElements: Div is needed for nested delete button
						<div
							key={meeting.id}
							className="history-item"
							onClick={() => handleMeetingClick(meeting.id)}
							onKeyDown={(e) => {
								if (e.key === "Enter" || e.key === " ") {
									handleMeetingClick(meeting.id);
								}
							}}
							role="button"
							tabIndex={0}
						>
							<div className="history-item-content">
								<div className="history-item-main">
									<h3 className="history-item-title">{meeting.title}</h3>
									<p className="history-item-date">{formatDate(meeting.date)}</p>
								</div>
								<div className="history-item-meta">
									<span className="history-item-duration">
										⏱ {formatDuration(meeting.duration_seconds)}
									</span>
								</div>
							</div>
							<button
								type="button"
								className="history-item-delete"
								onClick={(e) => {
									e.stopPropagation();
									handleDeleteClick(meeting.id);
								}}
								aria-label={`Delete ${meeting.title}`}
							>
								🗑️
							</button>
						</div>
					))}
				</div>

				{/* Delete Confirmation Modal */}
				{deleteConfirmId && (
					// biome-ignore lint/a11y/noStaticElementInteractions: Modal overlay with intentional click behavior
					<div
						className="modal-overlay"
						onClick={cancelDelete}
						onKeyDown={(e) => {
							if (e.key === "Escape") cancelDelete();
						}}
						role="presentation"
					>
						{/* biome-ignore lint/a11y/noStaticElementInteractions: Modal content container */}
						<div
							className="modal-content delete-modal"
							onClick={(e) => e.stopPropagation()}
							onKeyDown={(e) => {
								if (e.key === "Escape") cancelDelete();
							}}
							role="presentation"
						>
							<h3>Delete Meeting?</h3>
							<p className="modal-description">
								Are you sure you want to delete this meeting? This action cannot be undone.
							</p>

							<div className="modal-actions">
								<button
									type="button"
									className="modal-button secondary"
									onClick={cancelDelete}
									disabled={isDeleting}
								>
									Cancel
								</button>
								<button
									type="button"
									className="modal-button danger"
									onClick={confirmDelete}
									disabled={isDeleting}
								>
									{isDeleting ? "Deleting..." : "Delete"}
								</button>
							</div>
						</div>
					</div>
				)}
			</div>
		</div>
	);
}

export default HistoryView;
