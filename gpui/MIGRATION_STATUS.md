# GPUI native migration status

## Scope and safety
- Target repository: https://github.com/hlabin/hide
- Work branch: `feature/gpui-native-migration`
- Keep `main` and the working Tauri implementation intact until native parity is demonstrated.
- Existing product in this repository is lpm (a project/terminal/agent manager), despite the Hide repository name. The migration must preserve **actual** lpm features.

## Completed
- [x] Verified GitHub writes with a sample commit on the migration branch.
- [x] Audited React/Tauri entry points, existing project and terminal layout, the Rust desktop crate, and the CLI project listing schema.
- [x] Created standalone GPUI Rust crate in `gpui/` based on published gpui-ce crate APIs.
- [x] Added initial project sidebar, selected project details, terminal-area placeholder, refresh, and light/dark toggle.
- [x] Matched initial light/dark chrome palette to `desktop/frontend/src/styles/globals.css`.
- [x] Added read-only Rust backend adapter through the existing `lpm list --json` contract; parses project names, running states and services.
- [x] Added unit tests for successful, empty and invalid CLI listing JSON.
- [x] Added project start/stop through existing `lpm start/stop <name> --json` control CLI (no duplicate daemon/session owner), wired into native project detail view. Added action verb and invalid-name tests.
- [x] Added `gpui/README.md` with build instructions and explicit limitations.
- [x] Parsed agent status counts and worktree flags from the existing CLI JSON; native project detail now shows agent summary, remote/worktree type, and per-project config errors.
- [x] Added service detail retrieval via `lpm project <name> --json` and service start/stop via `lpm service <service> <start|stop> -p <project> --json`; UI now renders real service rows with port/status and per-service controls. Verifies the full CLI acknowledgement (ok/project/service/op) and rejects wrong project/service/action in unit tests.
- [x] Hardened start/stop control acknowledgement: require `ok: true` AND an exact matching project name, with unit tests for wrong project, rejected action and malformed JSON.

## Build and test status
- [x] Added `.github/workflows/gpui.yml` for macOS `cargo check` and `cargo test` on migration changes. Runs were cancelled before any steps: https://github.com/hlabin/hide/actions/runs/42279310963 and https://github.com/hlabin/hide/actions/runs/42279301147.
- [ ] `cargo check --manifest-path gpui/Cargo.toml` (GitHub Actions macOS job cancelled before execution; Rust/Cargo unavailable in this execution container).
- [ ] `cargo test --manifest-path gpui/Cargo.toml` (GitHub Actions macOS job cancelled before execution; outcome not verified).
- [ ] Native smoke test on macOS and Linux/Windows (NOT RUN).
- [ ] Screenshot-based visual regression against running Tauri app (NOT RUN).
- [ ] Rust backend integration/PTY feature parity (NOT DONE).
- Current GPUI app is **experimental** and must not be described as working, error-free or release-ready without the above checks.

## Run 2 verification (2026-11-07)
- Source contract checked against `cli/src/list.rs`, `cli/src/start.rs`, and `cli/src/stop.rs` on the migration branch.
- Source changes committed to this branch; Cargo/Rust unavailable in the execution container, so the new unit tests and GPUI compilation remain **unrun**.
- Latest observed GitHub Actions runs for this branch ended `failure` with cancelled jobs; this run did not establish working macOS runner execution. No successful CI build is claimed.
- Next: restore CI runner availability and run `cargo check`/`cargo test`; address any compiler errors before claiming native functionality works. Then wire service-level controls and live updates.

## Blockers observed
- GitHub Actions runs 42279310963, 42279301147, 42279205217 and 42279205007 all finished with failure status and a **cancelled** macOS job; no build/test steps ran. Job logs were unavailable (404 BlobNotFound). This is not evidence of a Rust compilation error. Check repository Actions policy, available macOS runners, billing and job cancellation rules; rerun after restoring runner execution.
- This execution environment has no `cargo` installed and cannot reach GitHub directly via `git` (DNS resolution failed). Changes were pushed via the GitHub connector. Source compilation is still unverified.

## Run 3 progress (2026-11-07)
- Added service-level backend and GPUI controls in commits `ddb093b` and `6da51cc` on the migration branch.
- Verified CLI command shape and JSON field contracts by reading `cli/src/project.rs` and `cli/src/service_cmd.rs` on the same branch.
- `cargo` and `rustc` are absent in the current execution container; no compile/test success claimed. Service commands execute synchronously and may block the GPUI UI thread; move to background tasks once the crate compiles.
- Next: obtain a successful macOS CI build, fix GPUI API/borrow errors, then make service requests asynchronous and implement terminal/PTY state updates.


## Run 4 progress (2026-11-07)
- Added typed live pane metadata (`id`, `title`, `currentCommand`, `currentPath`, `pid`, `ended`) from the existing `lpm project <name> --json` contract, alongside services in one project snapshot. No new daemon or duplicate PTY ownership.
- Native GPUI project detail now lists pane titles and active commands/ended state; actual PTY output/input remains unimplemented and is clearly labeled.
- Added parser test for live panes, missing optional fields, and wrong-project responses. Source contract inspected in `cli/src/project.rs` on the migration branch.
- Committed changes `8664eea` and `6c9ddfa` to the migration branch.
- Validation: no local `cargo`/`rustc` available and `git` cannot resolve github.com in the execution container. GitHub Actions macOS workflow for commit `6c9ddfa` was **queued** at last inspection (run https://github.com/hlabin/hide/actions/runs/42311060187); older runs failed before compilation. No passing build or tests claimed.
- Next: inspect new CI run and fix any compile errors; move blocking CLI reads and mutations off the GPUI UI thread; implement a native PTY transport/renderer without duplicating the existing session owner.

## Next incremental milestone
1. Run `cargo check` with Rust >=1.95 in an environment with GPUI's native build requirements. Fix actual compile errors before expanding functionality.
2. Extract Tauri-independent backend functionality into a shared Rust crate or maintain a deliberate RPC bridge to the existing desktop daemon. Do **not** use duplicate ownership of sessions.
3. Validate project and service start/stop in a running native build; move CLI requests off the UI thread, then implement terminals and agent status with matching state and update delivery.
4. Match sidebar, project view, tabs, typography, spacing, window chrome, keyboard interactions, and theming against actual Tauri screenshots.
5. Expand to settings, automations, Git, connections, notes, notifications, multiple windows, mobile integration and remaining feature matrix.
6. Rewrite root `README.md` for the new architecture **only after** GPUI can truthfully be presented as the desktop product. Audit any videos for obsolescence before removal.
7. Run all platform builds/tests and visual regression; replace Tauri only when native parity is established.

## Recovery instructions
- On each scheduled iteration, read this file and inspect branch HEAD first.
- Make small, reviewable commits on this branch.
- For transient GitHub failures, retry a few times and refresh file SHAs before updates.
- If a build/permission/environment step fails, record its exact cause, pursue another safe independent task, and resume later.
- Never claim an unrun test passed or delete working Tauri code prematurely.
