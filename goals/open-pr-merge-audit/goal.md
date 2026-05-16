Audit all currently open EchoNote GitHub PRs against `main`, verify which ones are safe to merge, and merge only the PRs that have no blocking issues. Draft PRs and PRs with conflicts, pending/failing checks, risky dependency jumps, or local verification failures must remain open with a documented reason.

The shared understanding is in `facts.md`.

The execution plan is in `plan.md`.

Done condition: every open PR from the starting audit has either been merged into `main` or is listed in the final result with the concrete blocker that kept it open.
