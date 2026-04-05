import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

// Transcription progress event from Tauri
interface TranscriptionProgress {
	percentage: number;
	status: string;
}

// Summary response from Tauri
interface GenerateSummaryResponse {
	summary_id: number;
	key_points: string;
	decisions: string;
	action_items: string;
	duration_seconds: number;
}

type ProcessingStage =
	| "idle"
	| "transcribing"
	| "transcription_complete"
	| "generating_summary"
	| "complete";

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

	// Processing state
	const [processingStage, setProcessingStage] = useState<ProcessingStage>("idle");
	const [transcriptionProgress, setTranscriptionProgress] = useState<TranscriptionProgress>({
		percentage: 0,
		status: "",
	});
	const [transcriptionResult, setTranscriptionResult] = useState<{
		transcript_id: number;
		text: string;
	} | null>(null);
	const [processingError, setProcessingError] = useState<string | null>(null);
	const [currentMeetingId, setCurrentMeetingId] = useState<number | null>(null);

	const unlistenRef = useRef<UnlistenFn | null>(null);
	const recordingIntervalRef = useRef<number | null>(null);

	// Listen to transcription progress events
	useEffect(() => {
		const setupListener = async () => {
			const unlisten = await listen<TranscriptionProgress>("transcription-progress", (event) => {
				setTranscriptionProgress(event.payload);
			});
			unlistenRef.current = unlisten;
		};

		setupListener();

		return () => {
			if (unlistenRef.current) {
				unlistenRef.current();
			}
		};
	}, []);

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

	// Start transcription
	const startTranscription = async (meetingId: number, audioPath: string) => {
		setProcessingStage("transcribing");
		setTranscriptionProgress({ percentage: 0, status: "Loading model..." });

		try {
			const response = await invoke<
				ApiResponse<{ transcript_id: number; text: string; duration_seconds: number }>
			>("transcribe_audio_command", {
				meeting_id: meetingId,
				audio_path: audioPath,
			});

			if (response.success && response.data) {
				setTranscriptionResult({
					transcript_id: response.data.transcript_id,
					text: response.data.text,
				});
				setProcessingStage("transcription_complete");
			} else {
				setProcessingError(response.error || "Transcription failed");
				setProcessingStage("idle");
			}
		} catch (err) {
			setProcessingError(err instanceof Error ? err.message : "Transcription failed");
			setProcessingStage("idle");
		}
	};

	// Generate summary
	const generateSummary = async () => {
		if (!currentMeetingId || !transcriptionResult) return;

		setProcessingStage("generating_summary");
		setProcessingError(null);

		try {
			const response = await invoke<ApiResponse<GenerateSummaryResponse>>(
				"generate_summary_command",
				{
					meeting_id: currentMeetingId,
					transcript: transcriptionResult.text,
				},
			);

			if (response.success && response.data) {
				setProcessingStage("complete");
				// Navigate to meeting detail after a short delay
				setTimeout(() => {
					if (onMeetingCreated) {
						onMeetingCreated(currentMeetingId);
					}
				}, 500);
			} else {
				setProcessingError(response.error || "Failed to generate summary");
				setProcessingStage("transcription_complete");
			}
		} catch (err) {
			setProcessingError(err instanceof Error ? err.message : "Failed to generate summary");
			setProcessingStage("transcription_complete");
		}
	};

	// Skip summary generation
	const skipSummary = () => {
		if (currentMeetingId && onMeetingCreated) {
			onMeetingCreated(currentMeetingId);
		}
	};

	// Reset processing state
	const resetProcessing = () => {
		setProcessingStage("idle");
		setTranscriptionProgress({ percentage: 0, status: "" });
		setTranscriptionResult(null);
		setProcessingError(null);
		setCurrentMeetingId(null);
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
				setCurrentMeetingId(response.data.id);
				setShowTitleModal(false);
				setMeetingTitle("");
				setRecordingResult(null);
				setRecordingDuration(0);

				// Start transcription immediately after saving
				await startTranscription(response.data.id, request.audio_path);
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
			if (unlistenRef.current) {
				unlistenRef.current();
			}
		};
	}, []);

	const isRecording = recordingState === "recording";
	const isSaving = recordingState === "saving";
	const isProcessing = processingStage !== "idle" && processingStage !== "complete";

	// Show processing view
	if (isProcessing) {
		return (
			<div className="record-view">
				<div className="processing-container">
					<h2>Processing Meeting</h2>

					{processingStage === "transcribing" && (
						<div className="processing-step">
							<div className="processing-spinner" />
							<h3>Transcribing Audio</h3>
							<p className="processing-status">{transcriptionProgress.status}</p>
							<div className="progress-bar">
								<div
									className="progress-fill"
									style={{ width: `${transcriptionProgress.percentage}%` }}
								/>
							</div>
							<span className="progress-percentage">
								{Math.round(transcriptionProgress.percentage)}%
							</span>
						</div>
					)}

					{processingStage === "transcription_complete" && (
						<div className="processing-step">
							<div className="processing-complete-icon">✓</div>
							<h3>Transcription Complete</h3>
							<p className="processing-status">
								Transcript saved with {transcriptionResult?.text.split(" ").length || 0} words
							</p>

							{processingError && (
								<div className="processing-error">
									<span className="error-icon">⚠️</span>
									<span>{processingError}</span>
								</div>
							)}

							<div className="processing-actions">
								<button type="button" className="processing-button secondary" onClick={skipSummary}>
									Skip Summary
								</button>
								<button
									type="button"
									className="processing-button primary"
									onClick={generateSummary}
									disabled={!transcriptionResult}
								>
									Generate Summary
								</button>
							</div>
						</div>
					)}

					{processingStage === "generating_summary" && (
						<div className="processing-step">
							<div className="processing-spinner" />
							<h3>Generating Summary</h3>
							<p className="processing-status">
								AI is analyzing the transcript and extracting key points...
							</p>
						</div>
					)}
				</div>
			</div>
		);
	}

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
						disabled={isSaving || isProcessing}
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
				{!isRecording && !isSaving && !isProcessing && (
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
								onClick={() => {
									cancelSave();
									resetProcessing();
								}}
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
