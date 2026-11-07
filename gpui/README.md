# Hide native GPUI frontend (work in progress)

This directory is an **experimental native Rust GPUI frontend** for the existing Tauri 2 desktop app in `desktop/frontend`. Do not distribute it as a replacement yet.

## Build

Requires Rust 1.95+, Cargo, and the native platform dependencies for [gpui-ce](https://docs.rs/gpui-ce/latest/gpui/). From the repository root:

```sh
cd gpui
cargo test
cargo run
```

The prototype currently uses `lpm list --json` as a **read-only adapter** to an existing public Rust CLI/socket interface. Install the repo's `lpm` CLI and ensure the existing lpm desktop app is running where necessary, or set `HIDE_LPM_CLI` to an absolute CLI executable path.

This is not yet an embedded, Tauri-independent backend. The GPUI window can display real project names and service status, select a project, refresh, and toggle between color palettes copied from the Tauri stylesheet. It cannot yet control services or render PTYs. Missing CLI or backend connectivity is surfaced as an error.

## Source references

- Tauri shell: `desktop/frontend/src/App.tsx`
- Sidebar: `desktop/frontend/src/components/Sidebar.tsx`
- Theme tokens: `desktop/frontend/src/styles/globals.css`
- CLI JSON contract: `cli/src/list.rs`
- Tauri Rust backend: `desktop/frontend/src-tauri/src/lib.rs`

Track remaining migration work in [MIGRATION_STATUS.md](MIGRATION_STATUS.md).
