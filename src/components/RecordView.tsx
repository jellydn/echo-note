import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

// Tauri API response wrapper
interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

// Recording response from Tauri
interface RecordingResponse {
	file_path: string;
	duration_seconds: number;
	used_system_audio: boolean;
}

// Meeting response from Tauri
interface MeetingResponse {
	id: number;
	title: string;
	date: string;
	duration_seconds: number;
	audio_path: string;
	created_at: string;
}

type RecordingState = "idle" | "recording" | "saving" | "processing";

interface RecordViewProps {
	onMeetingCreated?: (meetingId: number) => void;
}

export function RecordView({ onMeetingCreated }: RecordViewProps) {
	const [recordingState, setRecordingState] = useState<RecordingState>("idle");
	const [recordingDuration, setRecordingDuration] = useState(0);
	const [showTitleModal, setShowTitleModal] = useState(false);
	const [meetingTitle, setMeetingTitle] = useState("");
	const [isSubmitting, setIsSubmitting] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [recordingResult, setRecordingResult] = useState<RecordingResponse | null>(null);

	const recordingIntervalRef = useRef<number | null>(null);

	// Generate default meeting title
	const generateDefaultTitle = useCallback(() => {
		const now = new Date();
		const dateStr = now.toLocaleDateString("en-US", {
			month: "short",
			day: "numeric",
		});
		const timeStr = now.toLocaleTimeString("en-US", {
			hour: "2-digit",
			minute: "2-digit",
			hour12: false,
		});
		return `Meeting — ${dateStr} ${timeStr}`;
	}, []);

	// Format duration as MM:SS
	const formatDuration = (seconds: number): string => {
		const mins = Math.floor(seconds / 60);
		const secs = Math.floor(seconds % 60);
		return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
	};

	// Start recording
	const startRecording = async () => {
		try {
			setError(null);
			const response = await invoke<ApiResponse<boolean>>("start_recording_command");

			if (response.success && response.data) {
				setRecordingState("recording");
				setRecordingDuration(0);

				// Start duration timer
				recordingIntervalRef.current = window.setInterval(() => {
					setRecordingDuration((prev) => prev + 1);
				}, 1000);
			} else {
				setError(response.error || "Failed to start recording");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to start recording");
		}
	};

	// Stop recording
	const stopRecording = async () => {
		// Clear the timer
		if (recordingIntervalRef.current) {
			clearInterval(recordingIntervalRef.current);
			recordingIntervalRef.current = null;
		}

		try {
			setRecordingState("saving");
			const response = await invoke<ApiResponse<RecordingResponse>>("stop_recording_command");

			if (response.success && response.data) {
				setRecordingResult(response.data);
				// Set default title and show modal
				setMeetingTitle(generateDefaultTitle());
				setShowTitleModal(true);
				setRecordingState("idle");
			} else {
				setError(response.error || "Failed to stop recording");
				setRecordingState("idle");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to stop recording");
			setRecordingState("idle");
		}
	};

	// Save meeting with title
	const saveMeeting = async () => {
		if (!recordingResult || !meetingTitle.trim()) return;

		setIsSubmitting(true);
		setError(null);

		try {
			const now = new Date();
			const request = {
				title: meetingTitle.trim(),
				date: now.toISOString(),
				duration_seconds: Math.round(recordingResult.duration_seconds),
				audio_path: recordingResult.file_path,
			};

			const response = await invoke<ApiResponse<MeetingResponse>>("create_meeting_command", {
				request,
			});

			if (response.success && response.data) {
				setShowTitleModal(false);
				setMeetingTitle("");
				setRecordingResult(null);
				setRecordingDuration(0);

				// Notify parent component
				if (onMeetingCreated) {
					onMeetingCreated(response.data.id);
				}
			} else {
				setError(response.error || "Failed to save meeting");
			}
		} catch (err) {
			setError(err instanceof Error ? err.message : "Failed to save meeting");
		} finally {
			setIsSubmitting(false);
		}
	};

	// Cancel and discard recording
	const cancelSave = () => {
		setShowTitleModal(false);
		setMeetingTitle("");
		setRecordingResult(null);
		setRecordingDuration(0);
	};

	// Cleanup on unmount
	useEffect(() => {
		return () => {
			if (recordingIntervalRef.current) {
				clearInterval(recordingIntervalRef.current);
			}
		};
	}, []);

	const isRecording = recordingState === "recording";
	const isSaving = recordingState === "saving";

	return (
		<div className="record-view">
			<div className="record-container">
				{/* Recording Status */}
				<div className={`recording-status ${isRecording ? "active" : ""}`}>
					{isRecording && (
						<>
							<div className="recording-indicator">
								<span className="recording-dot" />
							</div>
							<div className="recording-timer">{formatDuration(recordingDuration)}</div>
							<div className="recording-label">Recording in progress...</div>
						</>
					)}
					{isSaving && (
						<>
							<div className="saving-spinner" />
							<div className="recording-label">Saving recording...</div>
						</>
					)}
					{!isRecording && !isSaving && <div className="recording-label">Ready to record</div>}
				</div>

				{/* Main Record Button */}
				<div className="record-button-container">
					<button
						type="button"
						className={`record-button ${isRecording ? "recording" : ""}`}
						onClick={isRecording ? stopRecording : startRecording}
						disabled={isSaving}
						aria-label={isRecording ? "Stop recording" : "Start recording"}
					>
						{isRecording ? (
							<>
								<span className="record-button-icon">⏹</span>
								<span className="record-button-text">Stop Recording</span>
							</>
						) : (
							<>
								<span className="record-button-icon">🔴</span>
								<span className="record-button-text">Start Recording</span>
							</>
						)}
					</button>
				</div>

				{/* Instructions */}
				{!isRecording && !isSaving && (
					<div className="record-instructions">
						<p>Click the button above to start recording your meeting.</p>
						<p className="record-hint">
							The app will capture both microphone and system audio (if BlackHole is installed).
						</p>
					</div>
				)}

				{/* Error Display */}
				{error && (
					<div className="record-error">
						<span className="error-icon">⚠️</span>
						<span>{error}</span>
					</div>
				)}
			</div>

			{/* Title Input Modal */}
			{showTitleModal && (
				<div className="modal-overlay">
					<div className="modal-content">
						<h3>Save Recording</h3>
						<p className="modal-description">
							Your recording has been saved. Enter a title for this meeting.
						</p>

						<div className="modal-input-group">
							<label htmlFor="meeting-title">Meeting Title</label>
							<input
								type="text"
								id="meeting-title"
								value={meetingTitle}
								onChange={(e) => setMeetingTitle(e.target.value)}
								placeholder="Enter meeting title..."
								disabled={isSubmitting}
								onKeyDown={(e) => {
									if (e.key === "Enter" && meetingTitle.trim()) {
										saveMeeting();
									}
								}}
							/>
						</div>

						{recordingResult && (
							<div className="modal-info">
								<span>
									Duration: {formatDuration(Math.round(recordingResult.duration_seconds))}
								</span>
								{recordingResult.used_system_audio && (
									<span className="system-audio-badge">System audio included</span>
								)}
							</div>
						)}

						<div className="modal-actions">
							<button
								type="button"
								className="modal-button secondary"
								onClick={cancelSave}
								disabled={isSubmitting}
							>
								Discard
							</button>
							<button
								type="button"
								className="modal-button primary"
								onClick={saveMeeting}
								disabled={!meetingTitle.trim() || isSubmitting}
							>
								{isSubmitting ? "Saving..." : "Save Meeting"}
							</button>
						</div>
					</div>
				</div>
			)}
		</div>
	);
}

export default RecordView;
