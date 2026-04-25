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

interface Transcript {
	id: number;
	meeting_id: number;
	content: string;
	created_at: string;
}

interface Summary {
	id: number;
	meeting_id: number;
	key_points: string;
	decisions: string;
	action_items: string;
	created_at: string;
}

interface MeetingDetailViewProps {
	meetingId: number;
	onBack?: () => void;
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
		month: "long",
		day: "numeric",
		year: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	});
};

// Copy text to clipboard
const copyToClipboard = async (text: string): Promise<boolean> => {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return false;
	}
};

export function MeetingDetailView({ meetingId, onBack }: MeetingDetailViewProps) {
	const [meeting, setMeeting] = useState<Meeting | null>(null);
	const [transcript, setTranscript] = useState<Transcript | null>(null);
	const [summary, setSummary] = useState<Summary | null>(null);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);

	// Edit title state
	const [isEditingTitle, setIsEditingTitle] = useState(false);
	const [editedTitle, setEditedTitle] = useState("");
	const [isSavingTitle, setIsSavingTitle] = useState(false);

	// Copy state
	const [copiedSection, setCopiedSection] = useState<string | null>(null);

	// Fetch meeting data
	const fetchMeetingData = useCallback(async () => {
		setIsLoading(true);
		setError(null);

		try {
			// Fetch meeting details
			const meetingResponse = await invoke<ApiResponse<Meeting>>("get_meeting_command", {
				id: meetingId,
			});

			if (!meetingResponse.success || !meetingResponse.data) {
				setError(meetingResponse.error || "Meeting not found");
				return;
			}

			setMeeting(meetingResponse.data);
			setEditedTitle(meetingResponse.data.title);

			// Fetch transcript
			const transcriptResponse = await invoke<ApiResponse<Transcript | null>>(
				"get_transcript_by_meeting_command",
				{ meetingId },
			);
			if (transcriptResponse.success) {
				setTranscript(transcriptResponse.data);
			}

			// Fetch summary
			const summaryResponse = await invoke<ApiResponse<Summary | null>>(
				"get_summary_by_meeting_command",
				{ meetingId },
			);
			if (summaryResponse.success) {
				setSummary(summaryResponse.data);
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to load meeting details");
		} finally {
			setIsLoading(false);
		}
	}, [meetingId]);

	// Load data on mount
	useEffect(() => {
		fetchMeetingData();
	}, [fetchMeetingData]);

	// Handle title edit start
	const handleTitleClick = () => {
		if (!meeting) return;
		setIsEditingTitle(true);
		setEditedTitle(meeting.title);
	};

	// Handle title save
	const handleTitleSave = async () => {
		if (!meeting || editedTitle.trim() === meeting.title) {
			setIsEditingTitle(false);
			return;
		}

		setIsSavingTitle(true);
		try {
			const response = await invoke<ApiResponse<Meeting>>("update_meeting_command", {
				id: meetingId,
				title: editedTitle.trim(),
			});

			if (response.success && response.data) {
				setMeeting(response.data);
				setIsEditingTitle(false);
			} else {
				setError(response.error || "Failed to update title");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to update title");
		} finally {
			setIsSavingTitle(false);
		}
	};

	// Handle title cancel
	const handleTitleCancel = () => {
		if (meeting) {
			setEditedTitle(meeting.title);
		}
		setIsEditingTitle(false);
	};

	// Handle copy with feedback
	const handleCopy = async (text: string, section: string) => {
		const success = await copyToClipboard(text);
		if (success) {
			setCopiedSection(section);
			setTimeout(() => setCopiedSection(null), 2000);
		}
	};

	// Parse key points from summary text
	const parseKeyPoints = (keyPoints: string): string[] => {
		return keyPoints
			.split("\n")
			.map((line) => line.trim())
			.filter((line) => line.length > 0 && (line.startsWith("-") || line.startsWith("*")))
			.map((line) => line.replace(/^[-*]\s*/, ""));
	};

	// Parse decisions from summary text
	const parseDecisions = (decisions: string): string[] => {
		return decisions
			.split("\n")
			.map((line) => line.trim())
			.filter((line) => line.length > 0 && (line.startsWith("-") || line.startsWith("*")))
			.map((line) => line.replace(/^[-*]\s*/, ""));
	};

	// Parse action items from summary text
	const parseActionItems = (actionItems: string): string[] => {
		return actionItems
			.split("\n")
			.map((line) => line.trim())
			.filter((line) => line.length > 0 && (line.startsWith("-") || line.startsWith("*")))
			.map((line) => line.replace(/^[-*]\s*/, ""));
	};

	if (isLoading) {
		return (
			<div className="meeting-detail-view">
				<div className="meeting-detail-container">
					<div className="meeting-detail-loading">
						<div className="meeting-detail-spinner" />
						<p>Loading meeting details...</p>
					</div>
				</div>
			</div>
		);
	}

	if (error || !meeting) {
		return (
			<div className="meeting-detail-view">
				<div className="meeting-detail-container">
					<div className="meeting-detail-error">
						<span className="error-icon">⚠️</span>
						<span>{error || "Meeting not found"}</span>
					</div>
					{onBack && (
						<button type="button" className="back-button" onClick={onBack}>
							← Back to History
						</button>
					)}
				</div>
			</div>
		);
	}

	return (
		<div className="meeting-detail-view">
			<div className="meeting-detail-container">
				{/* Header with back button */}
				<div className="meeting-detail-header">
					{onBack && (
						<button type="button" className="back-button" onClick={onBack}>
							← Back
						</button>
					)}
				</div>

				{/* Meeting Title and Meta */}
				<div className="meeting-detail-title-section">
					{isEditingTitle ? (
						<div className="title-edit-container">
							<input
								type="text"
								value={editedTitle}
								onChange={(e) => setEditedTitle(e.target.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter") handleTitleSave();
									if (e.key === "Escape") handleTitleCancel();
								}}
								// biome-ignore lint/a11y/noAutofocus: Needed for inline editing UX
								autoFocus
								className="title-edit-input"
								disabled={isSavingTitle}
							/>
							<div className="title-edit-actions">
								<button
									type="button"
									className="title-edit-button save"
									onClick={handleTitleSave}
									disabled={isSavingTitle}
								>
									✓
								</button>
								<button
									type="button"
									className="title-edit-button cancel"
									onClick={handleTitleCancel}
									disabled={isSavingTitle}
								>
									✕
								</button>
							</div>
						</div>
					) : (
						<div className="title-display-container">
							<h1 className="meeting-detail-title">{meeting.title}</h1>
							<button
								type="button"
								className="title-edit-trigger"
								onClick={handleTitleClick}
								title="Edit title"
							>
								✏️
							</button>
						</div>
					)}
					<div className="meeting-detail-meta">
						<span className="meta-item">📅 {formatDate(meeting.date)}</span>
						<span className="meta-separator">•</span>
						<span className="meta-item">⏱ {formatDuration(meeting.duration_seconds)}</span>
					</div>
				</div>

				{/* Content Grid */}
				<div className="meeting-detail-content">
					{/* Summary Card */}
					<div className="meeting-detail-card summary-card">
						<div className="card-header">
							<h2>Summary</h2>
							{summary && (
								<button
									type="button"
									className={`copy-button ${copiedSection === "summary" ? "copied" : ""}`}
									onClick={() =>
										handleCopy(
											`Key Points:\n${summary.key_points}\n\nDecisions:\n${summary.decisions}\n\nAction Items:\n${summary.action_items}`,
											"summary",
										)
									}
								>
									{copiedSection === "summary" ? "✓ Copied" : "📋 Copy"}
								</button>
							)}
						</div>

						{summary ? (
							<div className="summary-sections">
								{/* Key Points */}
								<div className="summary-section">
									<h3>Key Points</h3>
									<ul className="summary-list">
										{parseKeyPoints(summary.key_points).map((point, index) => (
											// biome-ignore lint/suspicious/noArrayIndexKey: Static list from parsed content
											<li key={index}>{point}</li>
										))}
									</ul>
									{parseKeyPoints(summary.key_points).length === 0 && (
										<p className="empty-section">{summary.key_points}</p>
									)}
								</div>

								{/* Decisions */}
								<div className="summary-section">
									<h3>Decisions</h3>
									<ul className="summary-list">
										{parseDecisions(summary.decisions).map((decision, index) => (
											// biome-ignore lint/suspicious/noArrayIndexKey: Static list from parsed content
											<li key={index}>{decision}</li>
										))}
									</ul>
									{parseDecisions(summary.decisions).length === 0 && (
										<p className="empty-section">{summary.decisions}</p>
									)}
								</div>

								{/* Action Items */}
								<div className="summary-section">
									<h3>Action Items</h3>
									<ul className="summary-list">
										{parseActionItems(summary.action_items).map((item, index) => (
											// biome-ignore lint/suspicious/noArrayIndexKey: Static list from parsed content
											<li key={index}>{item}</li>
										))}
									</ul>
									{parseActionItems(summary.action_items).length === 0 && (
										<p className="empty-section">{summary.action_items}</p>
									)}
								</div>
							</div>
						) : (
							<div className="no-content">
								<p>No summary generated for this meeting.</p>
							</div>
						)}
					</div>

					{/* Transcript Card */}
					<div className="meeting-detail-card transcript-card">
						<div className="card-header">
							<h2>Transcript</h2>
							{transcript && (
								<button
									type="button"
									className={`copy-button ${copiedSection === "transcript" ? "copied" : ""}`}
									onClick={() => transcript && handleCopy(transcript.content, "transcript")}
								>
									{copiedSection === "transcript" ? "✓ Copied" : "📋 Copy"}
								</button>
							)}
						</div>

						<div className="transcript-content">
							{transcript ? (
								<div className="transcript-text">{transcript.content}</div>
							) : (
								<div className="no-content">
									<p>No transcript available for this meeting.</p>
								</div>
							)}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}

export default MeetingDetailView;
