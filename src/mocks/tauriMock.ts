import type { InvokeArgs } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";

/**
 * Browser-only mock of the Tauri IPC surface.
 *
 * EchoNote is a Tauri app: every `invoke()` call routes through
 * `window.__TAURI_INTERNALS__.invoke` and every `listen()`/`emit()` through the
 * `plugin:event|*` commands. When the frontend is loaded in a plain browser
 * (like the Freebuff preview) there is no Rust backend, so all of those calls
 * throw. This module installs the official `mockIPC` from `@tauri-apps/api`
 * with `shouldMockEvents: true` to provide a realistic, stateful stand-in:
 *
 * - Every command the views call returns `ApiResponse<T>`-shaped data.
 * - A small in-memory store holds meetings, transcripts, summaries and
 *   settings, so Record → save → transcribe → summarize → History flows work
 *   end to end.
 * - Progress events (`transcription-progress`, `whisper-download-progress`,
 *   `diarization-download-progress`) are emitted during simulated async work.
 *
 * It only activates when no real Tauri shell is present, so native builds are
 * completely unaffected.
 */

// ---------------------------------------------------------------------------
// Types (mirror of the shapes the views expect)
// ---------------------------------------------------------------------------

interface ApiResponse<T> {
	success: boolean;
	data: T | null;
	error: string | null;
}

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

interface MockConfig {
	/** When true, the app boots into the SetupWizard instead of the main shell. */
	firstLaunch: boolean;
	/** When false, the Record view shows the BlackHole install banner. */
	blackholeInstalled: boolean;
}

// ---------------------------------------------------------------------------
// Config + store
// ---------------------------------------------------------------------------

const config: MockConfig = {
	firstLaunch: false,
	blackholeInstalled: true,
};

// In-memory settings table (keys mirror SettingsView's SETTING_* constants).
const settings = new Map<string, string>([
	["audio_device", "MacBook Pro Microphone"],
	["whisper_model_size", "small"],
	["llm_provider", "ollama"],
	["api_key", ""],
	["api_endpoint", "http://localhost:11434"],
	["diarization_enabled", "true"],
	["diarization_threshold", "0.75"],
]);

const meetings: Meeting[] = [
	{
		id: 1,
		title: "Weekly Product Sync",
		date: "2026-07-30T09:00:00.000Z",
		duration_seconds: 43 * 60,
		audio_path: "/tmp/echonote/recordings/weekly-sync-2026-07-30.wav",
		created_at: "2026-07-30T09:43:00.000Z",
	},
	{
		id: 2,
		title: "Q2 Planning — Roadmap Review",
		date: "2026-07-27T14:00:00.000Z",
		duration_seconds: 62 * 60,
		audio_path: "/tmp/echonote/recordings/q2-roadmap-2026-07-27.wav",
		created_at: "2026-07-27T15:02:00.000Z",
	},
	{
		id: 3,
		title: "Client Onboarding Call — Acme Corp",
		date: "2026-07-24T10:30:00.000Z",
		duration_seconds: 28 * 60,
		audio_path: "/tmp/echonote/recordings/acme-onboarding-2026-07-24.wav",
		created_at: "2026-07-24T10:58:00.000Z",
	},
	{
		id: 4,
		title: "Sprint Retrospective",
		date: "2026-07-21T16:00:00.000Z",
		duration_seconds: 35 * 60,
		audio_path: "/tmp/echonote/recordings/sprint-retro-2026-07-21.wav",
		created_at: "2026-07-21T16:35:00.000Z",
	},
	{
		id: 5,
		title: "Design Review: New Onboarding Flow",
		date: "2026-07-17T11:00:00.000Z",
		duration_seconds: 51 * 60,
		audio_path: "/tmp/echonote/recordings/onboarding-design-2026-07-17.wav",
		created_at: "2026-07-17T11:51:00.000Z",
	},
	{
		id: 6,
		title: "1:1 with Priya — Career Growth",
		date: "2026-07-15T09:30:00.000Z",
		duration_seconds: 19 * 60,
		audio_path: "/tmp/echonote/recordings/priya-1on1-2026-07-15.wav",
		created_at: "2026-07-15T09:49:00.000Z",
	},
];

const transcripts: Transcript[] = [
	{
		id: 1,
		meeting_id: 1,
		content: [
			"[00:00] Alex: Alright, let's kick off the weekly product sync.",
			"[00:08] Priya: Good morning everyone. I'll start with the metrics.",
			"[00:15] Priya: Activation is up to 82% this week, a 4-point improvement over last week.",
			"[00:31] Sam: Nice. On the engineering side we shipped the audio capture fixes yesterday.",
			"[00:45] Alex: Great. Did that resolve the BlackHole dropouts people were seeing?",
			"[01:02] Sam: Mostly. There's still an edge case with the 2-hour buffer truncation we're tracking.",
			"[01:20] Priya: Noted. We should document that limit in the release notes.",
			"[01:38] Alex: Agreed. Let's also talk about the export button rollout.",
			"[01:52] Sam: Export to Markdown is live and the clipboard fallback works in the sandbox.",
			"[02:10] Priya: Storage usage UI looks good too — cleanup freed about 3 GB this week.",
			"[02:25] Alex: Perfect. Action items: Sam owns the truncation edge case, Priya drafts release notes.",
		].join("\n"),
		created_at: "2026-07-30T09:46:00.000Z",
	},
	{
		id: 2,
		meeting_id: 2,
		content: [
			"[00:00] Priya: Welcome to the roadmap review. We have six candidate initiatives for Q2.",
			"[00:18] Alex: Let's score them against the three strategic pillars: capture, understanding, and automation.",
			"[00:36] Sam: The speaker diarization work scores high on understanding — I'd move it to committed.",
			"[00:54] Priya: Agreed. The meeting search feature is also ready for a beta.",
			"[01:12] Alex: What about the summarization quality improvements?",
			"[01:28] Sam: We can ship the model switch feature, but the API provider integration needs more testing.",
			"[01:47] Priya: Let's commit to diarization, search beta, and the model switch; defer the API provider.",
			"[02:05] Alex: Good. Notes will be circulated by Friday.",
		].join("\n"),
		created_at: "2026-07-27T15:05:00.000Z",
	},
	{
		id: 3,
		meeting_id: 3,
		content: [
			"[00:00] Sam: Hi everyone, thanks for the time. Today is about getting Acme set up with EchoNote.",
			"[00:14] Dana: Great. We've got 40 people who run weekly standups in Zoom.",
			"[00:26] Sam: Perfect. The BlackHole loopback makes system audio capture work out of the box.",
			"[00:41] Dana: And how do our teams get the transcripts afterward?",
			"[00:52] Sam: Meeting notes land in History with the summary, key points, and action items.",
			"[01:08] Dana: That's exactly what we need for our remote-first standups.",
			"[01:20] Sam: I'll send the setup guide and a license quote by end of day.",
		].join("\n"),
		created_at: "2026-07-24T11:01:00.000Z",
	},
	{
		id: 4,
		meeting_id: 4,
		content: [
			"[00:00] Alex: Let's run the retrospective. What went well this sprint?",
			"[00:10] Sam: The storage cleanup feature shipped on time and cut the recordings dir by half.",
			"[00:24] Priya: Also the search box in History — way faster to find past meetings now.",
			"[00:38] Alex: What didn't go well?",
			"[00:44] Sam: The transcription model download stalled once on a flaky network — no retry logic.",
			"[00:58] Priya: And the mic test still says 'good' even when the peak is borderline.",
			"[01:12] Alex: Both fair. Let's add download retries and a clearer mic test message next sprint.",
		].join("\n"),
		created_at: "2026-07-21T16:38:00.000Z",
	},
	{
		id: 5,
		meeting_id: 5,
		content: [
			"[00:00] Priya: Let's walk the new onboarding flow. Step one is the welcome screen.",
			"[00:12] Sam: I like the BlackHole check happening early — it surfaces the banner before recording.",
			"[00:28] Dana: The microphone test is nice, but the copy could mention system audio capture.",
			"[00:44] Priya: Agreed, we'll update the copy. What about the settings handoff after setup?",
			"[00:58] Sam: Setup completion persists the flag and lands you on the Record view.",
			"[01:14] Dana: Solid. One more pass on the empty states and we're good to ship.",
		].join("\n"),
		created_at: "2026-07-17T11:54:00.000Z",
	},
	{
		id: 6,
		meeting_id: 6,
		content: [
			"[00:00] Alex: Thanks for making time, Priya. How are things going overall?",
			"[00:10] Priya: Honestly, great — I'm enjoying the diarization work and it's shipping to real users.",
			"[00:26] Alex: Any friction?",
			"[00:31] Priya: I'd like to own the search feature end to end next quarter.",
			"[00:44] Alex: That's yours. Let's also look at a mentorship path for you with the transcript pipeline.",
			"[01:02] Priya: I'd really appreciate that.",
			"[01:10] Alex: I'll set it up with the ML team lead this week.",
		].join("\n"),
		created_at: "2026-07-15T09:52:00.000Z",
	},
];

const summaries: Summary[] = [
	{
		id: 1,
		meeting_id: 1,
		key_points: [
			"- Activation reached 82%, up 4 points week-over-week",
			"- Audio capture fixes shipped; BlackHole dropout edge case remains",
			"- Export to Markdown shipped with clipboard fallback",
			"- Storage cleanup freed roughly 3 GB this week",
		].join("\n"),
		decisions: [
			"- Document the 2-hour buffer limit in release notes",
			"- Ship the export button in the next patch release",
		].join("\n"),
		action_items: [
			"- Sam: fix the 2-hour buffer truncation edge case",
			"- Priya: draft release notes for the buffer limit",
		].join("\n"),
		created_at: "2026-07-30T09:47:00.000Z",
	},
	{
		id: 2,
		meeting_id: 2,
		key_points: [
			"- Six candidate initiatives scored against three strategic pillars",
			"- Speaker diarization moved to committed",
			"- Meeting search ready for beta",
			"- API provider integration needs more testing",
		].join("\n"),
		decisions: [
			"- Commit to diarization, search beta, and the model switch",
			"- Defer the API provider integration to next quarter",
		].join("\n"),
		action_items: [
			"- Priya: circulate roadmap notes by Friday",
			"- Sam: schedule the search beta with the design team",
		].join("\n"),
		created_at: "2026-07-27T15:06:00.000Z",
	},
	{
		id: 3,
		meeting_id: 3,
		key_points: [
			"- Acme Corp onboarding: 40 people running weekly Zoom standups",
			"- BlackHole loopback covers system audio capture out of the box",
			"- Meeting notes include transcript, summary, and action items",
		].join("\n"),
		decisions: ["- Send the setup guide and license quote by end of day"].join("\n"),
		action_items: ["- Sam: email setup guide and quote to Dana"].join("\n"),
		created_at: "2026-07-24T11:02:00.000Z",
	},
	{
		id: 4,
		meeting_id: 4,
		key_points: [
			"- Storage cleanup shipped and halved the recordings directory",
			"- History search made finding past meetings much faster",
			"- Model download lacks retry logic on flaky networks",
			"- Mic test message is ambiguous on borderline peaks",
		].join("\n"),
		decisions: [
			"- Add download retry logic next sprint",
			"- Make the mic test message clearer",
		].join("\n"),
		action_items: [
			"- Sam: add download retries for whisper models",
			"- Priya: refine mic test copy and thresholds",
		].join("\n"),
		created_at: "2026-07-21T16:39:00.000Z",
	},
	{
		id: 5,
		meeting_id: 5,
		key_points: [
			"- BlackHole check runs early in onboarding, surfacing the banner before recording",
			"- Microphone test could mention system audio capture",
			"- Setup completion persists the flag and lands on the Record view",
		].join("\n"),
		decisions: [
			"- Update mic test copy to mention system audio capture",
			"- One more pass on empty states before shipping",
		].join("\n"),
		action_items: ["- Dana: final review of empty states and copy"].join("\n"),
		created_at: "2026-07-17T11:55:00.000Z",
	},
	{
		id: 6,
		meeting_id: 6,
		key_points: [
			"- Priya is enjoying the diarization work and it is shipping to real users",
			"- Priya wants to own the search feature end to end next quarter",
			"- Alex to set up a mentorship path on the transcript pipeline",
		].join("\n"),
		decisions: [
			"- Priya owns the search feature next quarter",
			"- Start a mentorship path with the ML team lead",
		].join("\n"),
		action_items: ["- Alex: introduce Priya to the ML team lead this week"].join("\n"),
		created_at: "2026-07-15T09:53:00.000Z",
	},
];

// Whisper model catalog, shared by the model-list and download commands.
interface WhisperModelInfo {
	size: string;
	filename: string;
	expected_size: number;
	is_downloaded: boolean;
}

const WHISPER_MODELS: WhisperModelInfo[] = [
	{ size: "tiny", filename: "ggml-tiny.bin", expected_size: 75_080_000, is_downloaded: true },
	{ size: "base", filename: "ggml-base.bin", expected_size: 142_000_000, is_downloaded: true },
	{ size: "small", filename: "ggml-small.bin", expected_size: 466_000_000, is_downloaded: true },
	{
		size: "medium",
		filename: "ggml-medium.bin",
		expected_size: 1_535_000_000,
		is_downloaded: false,
	},
	{
		size: "large-v3",
		filename: "ggml-large-v3.bin",
		expected_size: 2_970_000_000,
		is_downloaded: false,
	},
];

// Diarization model, shared by the status and download commands.
const DIARIZATION_MODEL = {
	id: "pyannote/speaker-diarization-3.1",
	filename: "diarize-3.1.onnx",
	expected_size: 173_000_000,
	is_downloaded: true,
	model_path: "/tmp/echonote/models/diarize-3.1.onnx",
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const delay = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

const ok = <T>(data: T): ApiResponse<T> => ({ success: true, data, error: null });

const nextId = (items: Array<{ id: number }>): number =>
	items.reduce((max, item) => Math.max(max, item.id), 0) + 1;

const findMeeting = (id: number): Meeting | undefined => meetings.find((m) => m.id === id);

const findTranscript = (meetingId: number): Transcript | undefined =>
	transcripts.find((t) => t.meeting_id === meetingId);

const findSummary = (meetingId: number): Summary | undefined =>
	summaries.find((s) => s.meeting_id === meetingId);

const sortedMeetings = (): Meeting[] => [...meetings].sort((a, b) => b.id - a.id);

// Resolve a whisper model by size, defaulting to "small" for unknown sizes.
const findWhisperModel = (size: string): WhisperModelInfo =>
	WHISPER_MODELS.find((m) => m.size === size) ??
	WHISPER_MODELS.find((m) => m.size === "small") ??
	WHISPER_MODELS[0];

const meetingNotFound = (): ApiResponse<never> => ({
	success: false,
	data: null,
	error: "Meeting not found",
});

// Build a plausible multi-speaker transcript for a freshly recorded meeting.
const buildTranscriptFor = (title: string, durationSeconds: number): string => {
	const totalMinutes = Math.max(1, Math.round(durationSeconds / 60));
	const minutes = Math.min(totalMinutes, 3);
	const lines = [
		`[00:00] Speaker A: Thanks for joining — this session is about ${title}.`,
		"[00:12] Speaker B: Great. Let's set the agenda and capture the key decisions as we go.",
	];
	for (let i = 1; i <= minutes; i++) {
		const tag = i % 2 === 0 ? "Speaker B" : "Speaker A";
		const minuteLabel = String(i).padStart(2, "0");
		lines.push(
			`[${minuteLabel}:00] ${tag}: Moving to the next point — let's make sure we note any open questions.`,
		);
	}
	lines.push(
		`[${String(minutes + 1).padStart(2, "0")}:00] Speaker A: That wraps it up. Action items will be in the summary.`,
	);
	return lines.join("\n");
};

const buildSummaryFor = (title: string, meetingId: number): Summary => ({
	id: nextId(summaries),
	meeting_id: meetingId,
	key_points: [
		`- Discussed "${title}" with agreed next steps`,
		"- Captured the transcript and generated this summary automatically",
		"- No open blockers identified during the session",
	].join("\n"),
	decisions: [
		"- Track open questions in the meeting notes",
		"- Follow up on action items by end of week",
	].join("\n"),
	action_items: [
		"- Owner: review the transcript and confirm action items",
		"- Owner: share the summary with the meeting attendees",
	].join("\n"),
	created_at: new Date().toISOString(),
});

// Export a meeting as Markdown.
const buildMarkdownExport = (meeting: Meeting): string => {
	const transcript = findTranscript(meeting.id);
	const summary = findSummary(meeting.id);
	const date = new Date(meeting.date);
	const sections = [
		`# ${meeting.title}`,
		"",
		`- **Date:** ${date.toISOString()}`,
		`- **Duration:** ${Math.round(meeting.duration_seconds / 60)} min`,
		"",
		"## Transcript",
		"",
		transcript?.content ?? "_No transcript available._",
	];
	if (summary) {
		sections.push(
			"",
			"## Key Points",
			"",
			summary.key_points,
			"",
			"## Decisions",
			"",
			summary.decisions,
			"",
			"## Action Items",
			"",
			summary.action_items,
		);
	}
	return sections.join("\n");
};

const slugify = (title: string): string =>
	title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-+|-+$/g, "");

// Simulated async work that reports progress over time.
const emitProgress = async (
	eventName: string,
	buildPayload: (fraction: number) => Record<string, unknown>,
	steps = 5,
): Promise<void> => {
	for (let step = 1; step <= steps; step++) {
		await delay(350);
		const fraction = step / steps;
		await emit(eventName, buildPayload(fraction));
	}
};

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

const handleCommand = async (cmd: string, args?: InvokeArgs): Promise<unknown> => {
	// Narrow the union InvokeArgs down to the plain object the views always pass.
	const a = (args ?? {}) as Record<string, unknown>;
	switch (cmd) {
		// --- First launch / setup ---
		case "check_first_launch_status_command":
			return ok(config.firstLaunch);
		case "complete_first_launch_setup_command":
			config.firstLaunch = false;
			return ok(true);

		// --- BlackHole ---
		case "check_blackhole_status_command":
			return ok({
				installed: config.blackholeInstalled,
				device_name: config.blackholeInstalled ? "BlackHole 2ch" : null,
			});
		case "auto_install_blackhole_command":
		case "install_blackhole_command":
		case "install_blackhole_homebrew_command":
		case "install_blackhole_bundled_command":
			config.blackholeInstalled = true;
			return cmd === "auto_install_blackhole_command"
				? ok({
						success: true,
						method: "bundled",
						message: "BlackHole installed successfully from bundled package",
					})
				: ok(true);
		case "check_homebrew_status_command":
			return ok(true);

		// --- Recording ---
		case "test_microphone_command":
			return ok(0.42); // peak level → "good" (> 0.01)
		case "start_recording_command":
			return ok(true);
		case "stop_recording_command": {
			const duration_seconds = Math.max(1, Math.round(47 + Math.random() * 30));
			return ok({
				file_path: `/tmp/echonote/recordings/recording-${Date.now()}.wav`,
				duration_seconds,
				used_system_audio: config.blackholeInstalled,
				system_audio_error: null,
				audio_truncated: false,
			});
		}

		// --- Meetings ---
		case "list_meetings_command":
			return ok(sortedMeetings());
		case "search_meetings_command": {
			const query = String(a.query ?? "")
				.trim()
				.toLowerCase();
			if (!query) return ok(sortedMeetings());
			return ok(
				sortedMeetings().filter((meeting) => {
					const transcript = findTranscript(meeting.id);
					return (
						meeting.title.toLowerCase().includes(query) ||
						(transcript?.content.toLowerCase().includes(query) ?? false)
					);
				}),
			);
		}
		case "get_meeting_command": {
			const id = Number(a.id);
			const meeting = findMeeting(id);
			return meeting ? ok(meeting) : meetingNotFound();
		}
		case "create_meeting_command": {
			const request = (a.request ?? {}) as Record<string, unknown>;
			const now = new Date().toISOString();
			const meeting: Meeting = {
				id: nextId(meetings),
				title: String(request.title ?? "Untitled meeting"),
				date: String(request.date ?? now),
				duration_seconds: Number(request.duration_seconds ?? 0),
				audio_path: String(request.audio_path ?? ""),
				created_at: now,
			};
			meetings.push(meeting);
			return ok(meeting);
		}
		case "update_meeting_command": {
			const id = Number(a.id);
			const meeting = findMeeting(id);
			if (!meeting) return meetingNotFound();
			meeting.title = String(a.title ?? meeting.title);
			return ok(meeting);
		}
		case "delete_meeting_command": {
			const id = Number(a.id);
			const index = meetings.findIndex((m) => m.id === id);
			if (index === -1) return meetingNotFound();
			meetings.splice(index, 1);
			const tIndex = transcripts.findIndex((t) => t.meeting_id === id);
			if (tIndex !== -1) transcripts.splice(tIndex, 1);
			const sIndex = summaries.findIndex((s) => s.meeting_id === id);
			if (sIndex !== -1) summaries.splice(sIndex, 1);
			return ok(true);
		}

		// --- Transcripts ---
		case "get_transcript_by_meeting_command": {
			const meetingId = Number(a.meetingId);
			return ok(findTranscript(meetingId) ?? null);
		}
		case "transcribe_audio_command": {
			const meetingId = Number(a.meetingId);
			await emitProgress(
				"transcription-progress",
				(fraction) => ({
					percentage: Math.round(fraction * 100),
					status:
						fraction < 0.3
							? "Loading model..."
							: fraction < 0.6
								? "Preprocessing audio..."
								: fraction < 0.9
									? "Transcribing..."
									: "Finalizing segments...",
				}),
				6,
			);
			const meeting = findMeeting(meetingId);
			const duration_seconds = meeting?.duration_seconds ?? 180;
			const text = buildTranscriptFor(meeting?.title ?? "Recording", duration_seconds);
			const transcript: Transcript = {
				id: nextId(transcripts),
				meeting_id: meetingId,
				content: text,
				created_at: new Date().toISOString(),
			};
			transcripts.push(transcript);
			return ok({ transcript_id: transcript.id, text, duration_seconds });
		}

		// --- Summaries ---
		case "get_summary_by_meeting_command": {
			const meetingId = Number(a.meetingId);
			return ok(findSummary(meetingId) ?? null);
		}
		case "generate_summary_command": {
			const meetingId = Number(a.meetingId);
			await delay(900);
			const summary = buildSummaryFor(findMeeting(meetingId)?.title ?? "Meeting", meetingId);
			summaries.push(summary);
			return ok({
				summary_id: summary.id,
				key_points: summary.key_points,
				decisions: summary.decisions,
				action_items: summary.action_items,
				duration_seconds: findMeeting(meetingId)?.duration_seconds ?? 0,
			});
		}

		// --- Export ---
		case "export_meeting_command": {
			const meetingId = Number(a.meetingId);
			const meeting = findMeeting(meetingId);
			if (!meeting) return meetingNotFound();
			return ok({
				format: String(a.format ?? "markdown"),
				content: buildMarkdownExport(meeting),
				filename: `${slugify(meeting.title)}-${meeting.date.slice(0, 10)}.md`,
			});
		}

		// --- Settings ---
		case "get_setting_command": {
			const request = (a.request ?? {}) as Record<string, unknown>;
			const key = String(request.key ?? "");
			return ok(settings.get(key) ?? "");
		}
		case "set_setting_command": {
			const request = (a.request ?? {}) as Record<string, unknown>;
			const key = String(request.key ?? "");
			const value = String(request.value ?? "");
			settings.set(key, value);
			return ok(true);
		}

		// --- Devices & models ---
		case "list_audio_devices_command":
			return ok([
				{ id: "BuiltInMicrophoneDevice", name: "MacBook Pro Microphone" },
				{ id: "BlackHole_2ch", name: "BlackHole 2ch" },
				{ id: "ZoomAudioDevice", name: "ZoomAudioDevice" },
				{ id: "USB_Mic", name: "External USB Microphone" },
			]);
		case "list_whisper_models_command":
			return ok(
				WHISPER_MODELS.map((model) => ({
					...model,
					actual_size: model.is_downloaded ? model.expected_size : null,
				})),
			);
		case "check_diarization_status_command":
			return ok({
				...DIARIZATION_MODEL,
				actual_size: DIARIZATION_MODEL.is_downloaded ? DIARIZATION_MODEL.expected_size : null,
			});
		case "download_whisper_model_command": {
			const modelSize = String(a.modelSize ?? "small");
			const model = findWhisperModel(modelSize);
			await emitProgress(
				"whisper-download-progress",
				(fraction) => ({
					model_size: model.size,
					bytes_downloaded: Math.round(model.expected_size * fraction),
					total_bytes: model.expected_size,
					percentage: Math.round(fraction * 100),
				}),
				8,
			);
			return ok(`Downloaded ${model.size} model (${modelSize})`);
		}
		case "download_diarization_model_command": {
			const total = DIARIZATION_MODEL.expected_size;
			await emitProgress(
				"diarization-download-progress",
				(fraction) => ({
					model_id: DIARIZATION_MODEL.id,
					bytes_downloaded: Math.round(total * fraction),
					total_bytes: total,
					percentage: Math.round(fraction * 100),
				}),
				8,
			);
			return ok("Downloaded diarization model");
		}

		// --- LLM / Ollama ---
		case "check_ollama_status_command":
			return ok({ available: true, url: "http://localhost:11434" });

		// --- Storage ---
		case "get_storage_usage_command": {
			const total_bytes = meetings.reduce(
				(acc, meeting) => acc + meeting.duration_seconds * 32_000,
				250_000_000,
			);
			return ok({
				file_count: meetings.length * 2,
				total_bytes,
				recordings_dir: "/tmp/echonote/recordings",
			});
		}
		case "cleanup_old_recordings_command":
			return ok({ deleted_count: 3, freed_bytes: 524_288_000, retention_days: 30 });

		// --- Misc ---
		case "open_models_folder_command":
			return ok(true);
		case "plugin:opener|open_url": {
			// The real opener plugin launches the system browser. In a plain
			// browser, open a tab instead so links stay usable in the preview.
			const url = String(a.url ?? "");
			if (url) window.open(url, "_blank", "noopener");
			return null;
		}

		default:
			console.warn(`[tauri-mock] Unhandled command: ${cmd}`, args);
			return ok(null);
	}
};

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

const hasRealTauri = (): boolean => {
	const internals = (window as unknown as { __TAURI_INTERNALS__?: { invoke?: unknown } })
		.__TAURI_INTERNALS__;
	return typeof internals?.invoke === "function";
};

let installed = false;

/**
 * Install the browser mock. Safe to call multiple times; no-op when a real
 * Tauri runtime is present or when already installed.
 */
export function installTauriMock(): void {
	if (installed || hasRealTauri()) return;
	installed = true;
	mockIPC(handleCommand, { shouldMockEvents: true });
	console.info("[tauri-mock] Browser preview active — serving mocked Tauri commands.");
}
