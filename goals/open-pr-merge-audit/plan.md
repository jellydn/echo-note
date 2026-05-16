## Solution Approach

Audit the live open PR list against `main`, separate merge candidates from blocked PRs, locally verify each candidate with checks matched to its change type, and merge only PRs that remain clean at the moment of merge. Recheck PR state after each merge because dependency PRs and feature branches can change mergeability as `main` moves.

## Ordered Steps

1. Capture the starting state.
   - Systems: GitHub PR metadata, local git state.
   - Commands/checks: `git status --short`, `git fetch origin`, `gh pr list --state open --json number,title,headRefName,baseRefName,isDraft,mergeable,reviewDecision,statusCheckRollup,author,updatedAt,url`.
   - Verification: The audit has a numbered list of every open PR targeting `main`, including draft status, mergeability, and check state.

2. Classify PRs before local checkout.
   - Systems: GitHub checks, review state, PR branch metadata.
   - Commands/checks: `gh pr view <number> --json number,title,body,files,commits,reviews,reviewDecision,statusCheckRollup,isDraft,mergeable,baseRefName,headRefName`.
   - Verification: Draft PRs, non-`main` PRs, pending/failing checks, and non-mergeable PRs are marked blocked before any merge attempt.

3. Review dependency PRs in dependency-aware order.
   - Systems: `package.json`, `bun.lock`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, Tauri/Vite/TypeScript/Rust dependency compatibility.
   - PRs currently expected in this group: Renovate PRs for TypeScript, Vite, `@vitejs/plugin-react`, Tauri, `uuid`, `whisper-rs`, `reqwest`, `cpal`, and `tokio`.
   - Commands/checks: inspect `gh pr diff <number>`, compare overlapping lockfile updates, and prefer merging smaller compatible updates before major-version jumps.
   - Verification: Each dependency PR has an explicit decision: merge candidate, superseded by another PR, blocked by major-version risk, or blocked by local verification.

4. Review feature PRs against product behavior.
   - Systems: React UI in `src/`, Rust/Tauri backend in `src-tauri/`, PRD/Ralph context in `tasks/` and `scripts/ralph/`, prior EchoNote notes for BlackHole/system-audio and diarization behavior.
   - PRs currently expected in this group: `#40` speaker diarization and any open local-AI/BlackHole feature PR that is no longer draft at execution time.
   - Commands/checks: inspect `gh pr diff <number>`, review touched files, and compare behavior with the relevant PRD or prior repo convention.
   - Verification: Feature PRs have no obvious product-contract gaps, placeholder paths, unsafe audio-thread state, or unverified model/resource assumptions before they become merge candidates.

5. Locally verify each merge candidate.
   - Systems: TypeScript, Biome, Rust, Tauri build/check surface.
   - Commands/checks: for frontend-only changes run `just check-ts` and `just lint-ts`; for Rust/backend changes run `just check-rs`, `just lint-rs`, and targeted `just test-rs` where relevant; for broad or mixed changes run `just check` and `just lint`.
   - Verification: Candidate verification commands pass locally, or the PR is left open with the failing command and error summary.

6. Merge one clean PR at a time.
   - Systems: GitHub merge flow, `main` branch.
   - Commands/checks: `gh pr merge <number> --merge`, then `git fetch origin` and re-run the open PR list query.
   - Verification: The PR is closed/merged on GitHub, `main` advanced, and remaining PR mergeability is refreshed before the next decision.

7. Produce the final audit result.
   - Systems: GitHub PR list and local command output.
   - Commands/checks: final `gh pr list --state open --json number,title,isDraft,mergeable,statusCheckRollup,url`.
   - Verification: The final report lists merged PRs, unmerged PRs, the reason each unmerged PR remains open, and any verification command that could not be run.

## Risks Or Open Questions

- Some Renovate PRs are major-version upgrades, especially TypeScript and Vite-related updates, and may need to remain open even when GitHub checks are green.
- GitHub check state can change during the run; pending checks block merging until refreshed and passing.
- Feature PRs can require deeper manual review than dependency bumps, especially around BlackHole routing, audio thread ownership, Whisper/ONNX model paths, and app-resource assumptions.
- Local verification may be slow or require installed macOS/Tauri build prerequisites; failures should block merge rather than be ignored.
