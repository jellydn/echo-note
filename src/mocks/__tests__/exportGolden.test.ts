import { describe, expect, it } from "vitest";
import { buildMarkdownExport, buildTextExport, exportFilename } from "../exportRenderers";
import fixture from "./fixtures/export-meeting.json";
import goldens from "./fixtures/export-golden.json";

// Golden fidelity test (frontend CI job).
//
// The committed golden outputs (fixtures/export-golden.json) are generated
// from the *real* Rust renderers by `src-tauri/tests/export_goldens.rs`
// (`cargo test --test export_goldens -- --ignored regenerate_goldens`), and a
// CI guard there fails if they drift from the Rust side. Asserting the mock
// builders reproduce those goldens byte-for-byte here catches mock drift in
// the frontend job — the same guarantee the Rust differential test provides,
// but without needing the Rust toolchain.
//
// Regenerate goldens after a renderer change:
//   cargo test --test export_goldens -- --ignored regenerate_goldens

type GoldenCase = (typeof goldens)[keyof typeof goldens];

describe("export mock golden fidelity", () => {
	for (const c of fixture.cases) {
		it(`case '${c.name}' matches the committed golden output`, () => {
			if (!(c.name in goldens)) {
				throw new Error(
					`No committed golden output for case '${c.name}' — regenerate goldens with: cargo test --test export_goldens -- --ignored regenerate_goldens`,
				);
			}
			const transcript = c.transcript ?? undefined;
			const summary = c.summary ?? undefined;
			const actual: GoldenCase = {
				markdown: buildMarkdownExport(c.meeting, transcript, summary),
				text: buildTextExport(c.meeting, transcript, summary),
				filenameMd: exportFilename(c.meeting.title, c.meeting.date, "md"),
				filenameTxt: exportFilename(c.meeting.title, c.meeting.date, "txt"),
			};
			expect(actual).toEqual(goldens[c.name as keyof typeof goldens]);
		});
	}
});
