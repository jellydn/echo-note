# 🤖 Agent Guidelines for EchoNote

Privacy-first native desktop app: **Tauri v2 + React (TypeScript) + Rust**. Records meetings, transcribes via Whisper, summarizes via Ollama.

## Dev Commands

```bash
bun install                # Install frontend deps
cargo tauri dev            # Start app with hot reload
cargo tauri build          # Release build

# Quality gates (required before commits)
just check                 # cargo check && bun run typecheck
just lint                  # cargo clippy && biome check
just fmt                   # cargo fmt && biome --write
just pre-commit            # prek run --all-files
just test-rs               # Run Rust tests
```

## Project Structure

```
src/                      # React frontend (TypeScript)
src-tauri/                # Rust backend
  src/db/                 # SQLx queries
  migrations/             # SQLx migrations
  src/gen/                # Auto-generated (don't edit)
tasks/                    # PRD and planning docs
scripts/ralph/            # Ralph autonomous agent scripts
```

## Critical Context (Not Obvious)

- **Audio thread pattern**: `cpal::Stream` is not `Send`/`Sync` — use dedicated audio thread with message passing, never store in Tauri state
- **BlackHole**: Virtual audio driver for system audio capture. App detects via `is_blackhole_installed()`, shows UI warning if missing, mixes mic + system audio when available
- **Whisper models**: Stored in `~/Library/Application Support/echo-note/models/`
- **Ollama**: Runs on port 11434, falls back to cloud API if configured
- **Database**: SQLite at app data directory, use `sqlx::query!` macro

## Tauri Command Pattern

```rust
#[tauri::command]
async fn command_name(state: State<'_, AppState>) -> Result<String, String> {
    // Return Result<T, String>, never panic
    state.resource.do_something()
        .map_err(|e| format!("User-friendly error: {}", e))?;
    Ok("success".to_string())
}
```

## Quality Gates (Required)

1. `just check` — typecheck passes
2. `just lint` — no warnings
3. `just fmt` — format clean
4. Single story focus per commit (see PRD in tasks/)

## First-Launch Setup Pattern

When adding first-launch setup flows:

1. **Track state in settings table** - Use `first_launch_completed` setting to track if setup has run
2. **Check on app mount** - App.tsx checks `check_first_launch_status_command` before rendering main UI
3. **SetupWizard component** - Renders as full-screen overlay when `isFirstLaunch === true`
4. **Mark completion** - Call `complete_first_launch_setup_command` when user finishes setup
5. **Skip allowed** - Always provide skip option so users aren't blocked

```typescript
// App.tsx pattern
const [isFirstLaunch, setIsFirstLaunch] = useState<boolean | null>(null);

useEffect(() => {
  invoke<{ data: boolean }>("check_first_launch_status_command")
    .then(response => setIsFirstLaunch(response.data))
    .catch(() => setIsFirstLaunch(false));
}, []);

if (isFirstLaunch) {
  return <SetupWizard onComplete={() => setIsFirstLaunch(false)} />;
}
```

## Bundled Resource Pattern

For bundling external dependencies (like .pkg installers) with the app:

1. **Place in resources/** - Put files in `src-tauri/resources/`
2. **Configure tauri.conf.json** - Add to bundle.resources section:
   ```json
   "resources": {
     "resources/BlackHole2ch-0.6.0.pkg": "resources/BlackHole2ch-0.6.0.pkg"
   }
   ```
3. **Access via app_handle.path().resource_dir()** - Use Tauri's path API at runtime
4. **Privileged installation** - Use `osascript` with `with administrator privileges` for .pkg install

```rust
// Rust backend pattern for bundled resources
let resource_dir = app_handle.path().resource_dir()?;
let pkg_path = resource_dir.join("resources").join("Installer.pkg");

// Install with admin privileges via osascript
let escaped_path = pkg_path
  .to_string_lossy()
  .replace("\\", "\\\\")
  .replace("\"", "\\\"");
Command::new("osascript")
  .args(["-e", &format!(
    r#"do shell script "installer -pkg " & quoted form of "{}" & " -target /" with administrator privileges"#,
    escaped_path
  )])
  .status()?;
```

## Ralph Workflow

```bash
# In scripts/ralph/
cat prd.json              # Current user stories
cat progress.txt          # Learned patterns
# Work on passes: false stories, update progress.txt
```
