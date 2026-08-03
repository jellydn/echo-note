// Pure, dependency-free renderers that mirror the real Rust export renderers
// (src-tauri/src/export/mod.rs) byte-for-byte.
//
// Kept free of any Tauri or browser dependency so the differential fidelity
// test (src-tauri/tests/export_fidelity.rs) can run them directly under `bun`
// and compare against the real Rust output. The Tauri mock's export command
// handler imports these same functions, so the browser preview and the
// differential test always exercise the exact same code path.

export interface ExportMeeting {
	title: string;
	date: string;
	duration_seconds: number;
}

export interface ExportTranscript {
	content: string;
}

export interface ExportSummary {
	key_points: string;
	decisions: string;
	action_items: string;
}

// Format a UTC ISO date as "YYYY-MM-DD HH:MM UTC" to mirror the real export
// renderers' `%Y-%m-%d %H:%M UTC` output.
const formatUtcTimestamp = (iso: string): string => {
	const d = new Date(iso);
	const pad = (n: number): string => String(n).padStart(2, "0");
	return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())} UTC`;
};

// Format seconds as "Hh Mm SSs" to mirror the real export command's
// `format_duration` (e.g. 2580 -> "43m 00s", 3720 -> "1h 2m 00s").
// Durations are always non-negative (they come from recording lengths), so
// Math.floor matches Rust's truncating integer division for every real input.
const formatDuration = (seconds: number): string => {
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	const secs = seconds % 60;
	if (hours > 0) return `${hours}h ${minutes}m ${String(secs).padStart(2, "0")}s`;
	if (minutes > 0) return `${minutes}m ${String(secs).padStart(2, "0")}s`;
	return `${secs}s`;
};

// Mirror of the Rust `push_section` helper: "### {title}" then each non-empty
// trimmed line (bullets kept as-is, plain lines prefixed with "- "), skipping
// the section entirely when the trimmed content is empty. Ends with a trailing
// blank line.
const pushSection = (title: string, content: string): string => {
	const trimmed = content.trim();
	if (trimmed === "") return "";
	let out = `### ${title}\n\n`;
	for (const raw of trimmed.split("\n")) {
		const line = raw.trim();
		if (line === "") continue;
		out += line.startsWith("-") || line.startsWith("*") ? line : `- ${line}`;
		out += "\n";
	}
	out += "\n";
	return out;
};

// Export a meeting as Markdown, mirroring the real `render_meeting_markdown`:
// title/date/duration metadata, then "## Summary" with `###` bulleted sections
// (empty sections skipped), then "## Transcript". Trailing newlines match the
// Rust renderer exactly.
const buildMarkdownExport = (
	meeting: ExportMeeting,
	transcript?: ExportTranscript,
	summary?: ExportSummary,
): string => {
	let md = `# ${meeting.title}\n\n`;
	md += `- **Date:** ${formatUtcTimestamp(meeting.date)}\n`;
	md += `- **Duration:** ${formatDuration(meeting.duration_seconds)}\n\n`;
	if (summary) {
		md += "## Summary\n\n";
		md += pushSection("Key Points", summary.key_points);
		md += pushSection("Decisions", summary.decisions);
		md += pushSection("Action Items", summary.action_items);
		md += "\n";
	}
	if (transcript) {
		md += "## Transcript\n\n";
		md += transcript.content;
		md += "\n";
	}
	return md;
};

// Export a meeting as plain text, mirroring the real `render_meeting_text`:
// title/date/duration, then raw summary sections (never bulletized or skipped,
// even when empty), then the transcript. Trailing newlines match the Rust
// renderer exactly.
const buildTextExport = (
	meeting: ExportMeeting,
	transcript?: ExportTranscript,
	summary?: ExportSummary,
): string => {
	let text = `${meeting.title}\n`;
	text += `Date: ${formatUtcTimestamp(meeting.date)}\n`;
	text += `Duration: ${formatDuration(meeting.duration_seconds)}\n`;
	if (summary) {
		text += "\n--- Summary ---\n";
		text += `Key Points:\n${summary.key_points}\n`;
		text += `Decisions:\n${summary.decisions}\n`;
		text += `Action Items:\n${summary.action_items}\n`;
	}
	if (transcript) {
		text += "\n--- Transcript ---\n";
		text += transcript.content;
		text += "\n";
	}
	return text;
};

export { buildMarkdownExport, buildTextExport };
