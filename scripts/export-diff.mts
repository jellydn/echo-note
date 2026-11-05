import { readFileSync } from "node:fs";
import process from "node:process";
import { buildMarkdownExport, buildTextExport, exportFilename } from "../src/mocks/exportRenderers";

// Differential-fixture renderer used by src-tauri/tests/export_fidelity.rs.
// Reads a shared JSON fixture and prints the mock builders' output for every
// case in both formats (plus the export filenames), so the Rust test can
// compare byte-for-byte against the real renderers. Run: bun scripts/export-diff.mts <fixture.json>

interface FixtureCase {
	name: string;
	meeting: Parameters<typeof buildMarkdownExport>[0];
	transcript?: Parameters<typeof buildMarkdownExport>[1];
	summary?: Parameters<typeof buildMarkdownExport>[2];
}

const fixturePath = process.argv[2];
if (!fixturePath) {
	console.error("usage: bun scripts/export-diff.mts <fixture.json>");
	process.exit(1);
}

const fixture = JSON.parse(readFileSync(fixturePath, "utf8")) as { cases: FixtureCase[] };

const out: Record<
	string,
	{ markdown: string; text: string; filenameMd: string; filenameTxt: string }
> = {};
for (const c of fixture.cases) {
	out[c.name] = {
		markdown: buildMarkdownExport(c.meeting, c.transcript ?? undefined, c.summary ?? undefined),
		text: buildTextExport(c.meeting, c.transcript ?? undefined, c.summary ?? undefined),
		filenameMd: exportFilename(c.meeting.title, c.meeting.date, "md"),
		filenameTxt: exportFilename(c.meeting.title, c.meeting.date, "txt"),
	};
}
process.stdout.write(JSON.stringify(out));
