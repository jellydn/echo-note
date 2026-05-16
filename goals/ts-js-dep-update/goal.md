# TS/JS Dependency Update

Update all JS/TypeScript dependencies in `package.json` to their latest versions — safe minor/patch bumps as a single batch first, then one-at-a-time major version jumps in order of increasing risk. Each change is verified with the full quality gate and committed separately.

The shared understanding is in `facts.md`.

The execution plan is in `plan.md`.

## Done condition

Every JS/TS dependency in `package.json` is at its target version, all quality gates pass (`just check`, `just lint`, `bun run test:run`, `bun run build`), and the final state is committed. Any updates that couldn't be completed are documented in a `blocked.md` note with the specific failure reason.
