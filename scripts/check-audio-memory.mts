import { spawnSync } from "node:child_process";
import process from "node:process";
import { fileURLToPath } from "node:url";

// Opt-in macOS gate for #32. Measures only the production normalization/spool/
// slice path: fixed metadata and a deterministic embedder, no inference models.
// Run: bun scripts/check-audio-memory.mts
if (process.platform !== "darwin") {
	console.error("This RSS gate requires macOS /usr/bin/time -l.");
	process.exit(2);
}

const cwd = fileURLToPath(new URL("..", import.meta.url));
const build = spawnSync(
	"cargo",
	["test", "--manifest-path", "src-tauri/Cargo.toml", "--lib", "--no-run", "--message-format=json"],
	{ cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
);
if (build.status !== 0) {
	console.error(build.error?.message ?? build.stderr);
	process.exit(build.status ?? 1);
}
const executable = build.stdout
	.trim()
	.split("\n")
	.map((line) => JSON.parse(line))
	.find(
		(artifact) =>
			artifact.reason === "compiler-artifact" &&
			artifact.target.name === "echo_note_lib" &&
			artifact.executable,
	)?.executable;
if (!executable) throw new Error("Cargo did not produce the library test executable.");

const peaks: number[] = [];
for (const seconds of [300, 7200]) {
	const result = spawnSync(
		"/usr/bin/time",
		[
			"-l",
			executable,
			"--ignored",
			"--exact",
			"whisper::audio::tests::audio_memory_probe",
			"--nocapture",
		],
		{
			cwd,
			encoding: "utf8",
			env: { ...process.env, LC_ALL: "C", ECHO_AUDIO_SECONDS: String(seconds) },
		},
	);
	if (result.status !== 0) {
		console.error(result.error?.message ?? `${result.stdout}\n${result.stderr}`);
		process.exit(result.status ?? 1);
	}
	process.stdout.write(result.stdout);
	process.stderr.write(result.stderr);
	if (!result.stdout.includes(`audio_memory_probe seconds=${seconds} windows=${seconds / 30}`)) {
		throw new Error("The requested audio-path probe did not complete.");
	}
	const peak = result.stderr.match(/^\s*(\d+)\s+maximum resident set size\s*$/m);
	if (!peak) throw new Error("macOS time did not report peak RSS in bytes.");
	peaks.push(Number(peak[1]));
}

const growth = peaks[1] - peaks[0];
const limit = 64 * 1024 * 1024;
console.log(
	`short_peak_bytes=${peaks[0]} long_peak_bytes=${peaks[1]} growth_bytes=${growth} limit_bytes=${limit} result=${growth < limit ? "PASS" : "FAIL"}`,
);
if (growth >= limit) process.exit(1);
