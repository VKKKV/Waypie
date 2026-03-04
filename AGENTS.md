# AGENTS.md

Guidance for coding agents operating in this repository.
This file is intentionally practical and command-focused.

## Project Snapshot

- Language: Rust (edition 2021)
- Crate type: single binary (`waypie`)
- UI stack: GTK4 + Cairo + gtk4-layer-shell
- Async/runtime: Tokio global runtime + GTK main loop
- Config: TOML with hot-reload via `notify`
- IPC/desktop integration: zbus + system-tray + Wayland protocols

Current source layout (source of truth):

- `src/main.rs` - entrypoint, global runtime/app state
- `src/ui/` - UI composition, widget, hover/click pure logic, action routing
- `src/tray/` - tray watcher/client adapters
- `src/config.rs` - config structs, defaults, load/watch logic
- `src/color.rs` - color parsing and serde support
- `src/cursor.rs` - Wayland virtual pointer logic
- `src/utils.rs` - command spawning, geometry, config path helper
- `src/telemetry.rs` - debug counters gated by `WAYPIE_DEBUG`

Important UI module split:

- `src/ui/radial.rs` - widget orchestration and GTK event handling
- `src/ui/click_logic.rs` - pure click resolution helpers (unit-tested)
- `src/ui/hover_state.rs` - pure hover/transition decision helpers (unit-tested)
- `src/ui/action_dispatcher.rs` - action side effects (activate/context/dbus/command)

## Build, Lint, and Test Commands

Run from repository root: `/home/kita/code/waypie`.

### Build

- Debug build: `cargo build`
- Release build: `cargo build --release`
- Fast compile check: `cargo check`

Release binary path:

- `target/release/waypie`

### Lint and Formatting

- Format (apply): `cargo fmt`
- Format (check only): `cargo fmt -- --check`
- Lint: `cargo clippy`
- Strict lint gate (preferred before PR):
  - `cargo clippy --all-targets --all-features -- -D warnings`

Notes:

- There is no custom `rustfmt.toml` or `clippy.toml` in this repo.
- Default rustfmt style is expected unless a file clearly follows an existing local pattern.

### Tests

Unit tests are committed under `src/` modules. Use these commands:

- Run all tests: `cargo test`
- Run tests without capturing output: `cargo test -- --nocapture`
- Run a single test by name substring:
  - `cargo test test_name_substring`
- Run a single exact test name:
  - `cargo test exact_test_name -- --exact`
- Run tests in a specific integration test file:
  - `cargo test --test integration_test_file`
- Run one integration test function exactly:
  - `cargo test --test integration_test_file case_name -- --exact`

Useful when iterating on one module:

- `cargo test ui::hover_state` (name filter)
- `cargo test ui::click_logic` (name filter)
- `cargo test config::tests::` (module tests)
- `cargo test color::` (name filter)

### Run Locally

- Normal mode: `cargo run --release`
- Daemon mode (if supported by current args handling): `cargo run --release -- daemon`
- Debug logging for tray watcher issues:
  - `WAYPIE_DEBUG=1 cargo run --release`

## Code Style and Conventions

Follow existing patterns in touched files; do not reformat unrelated code.

### Imports

- Group imports by origin in this order:
  1. external crates
  2. `std`
  3. `crate::...`
- Prefer explicit imports over glob imports, except GTK prelude patterns already used:
  - `use gtk4::prelude::*;` is acceptable and common here.
- Keep imports minimal; remove unused imports.

### Formatting

- Use `cargo fmt` formatting.
- Keep functions and match arms readable; favor line breaks over dense one-liners.
- Avoid mass formatting churn in unrelated files.

### Types and Data Modeling

- Prefer strong enums/structs over stringly-typed branching.
  - Example pattern already used: `ui::menu_model::Action` enum variants.
- Use `Option<T>` for optional config/state rather than sentinel values.
- Use `#[derive(Debug, Clone, Deserialize, Serialize)]` for config models where appropriate.
- Keep config defaults explicit via dedicated default functions + serde defaults.

### Naming Conventions

- Types/enums/traits: `PascalCase`
- Functions/modules/variables: `snake_case`
- Constants/statics: `SCREAMING_SNAKE_CASE`
- Keep names domain-specific (`hover_parent_idx`, `menu_path`, `center_radius`).

### Error Handling

- Do not `unwrap()`/`expect()` in recoverable runtime paths.
- `expect()` is acceptable for hard initialization invariants already established in startup paths
  (e.g., global runtime not initialized).
- For utility boundaries, prefer `Result<T, E>` with context (`anyhow::Context` where used).
- For async/UI boundary code already returning `String` errors, keep consistency unless refactoring
  the entire call chain.
- Log actionable failures with context (`service`, `path`, operation name).

### Async and Concurrency

- Keep GTK/UI work on GTK main context.
- Spawn background async work on global Tokio runtime (`crate::RUNTIME`).
- Avoid blocking operations in UI callbacks.
- When sharing mutable state across tasks/threads, follow current `Arc<Mutex<_>>` / `Arc<RwLock<_>>`
  patterns already used by tray/config state.

### Config and Serialization

- Config file path is resolved via `directories::ProjectDirs`.
- New config fields must include sensible serde defaults to preserve backward compatibility.
- If changing config schema, update both parser structs and generated default config behavior.

### UI and Interaction

- Preserve radial menu interaction semantics:
  - center dead-zone behavior
  - hover timing delay before activation
  - inner/outer ring hit logic
- Keep pointer math helpers deterministic and testable when possible.
- Prefer putting pure decision logic in `ui::click_logic` / `ui::hover_state`, and keep GTK widget
  code in `ui::radial` focused on orchestration.

## Repository-Specific Rules from Copilot Instructions

The repo includes `.github/copilot-instructions.md`; align with it where still applicable:

- Build/release command expectation: `cargo build --release`
- Lint command expectation: `cargo clippy`
- Config defaults pattern: serde defaults via helper functions
- Color model expectation: normalized RGB/RGBA values in `0.0..=1.0`
- Use utility command execution helper (`spawn_app`/execution utility) instead of ad-hoc shell glue
- Debug env switch referenced by docs: `WAYPIE_DEBUG=1`

Important: some Copilot architecture notes appear older than current tree
(for example `hud/` vs current `ui/`). Prefer the current `src/` layout and code.

## Cursor Rules

- No `.cursorrules` file found.
- No `.cursor/rules/` directory found.

If Cursor rules are added later, treat them as high-priority repository policy and merge them into this file.

## Agent Workflow Expectations

- Before finalizing changes: run `cargo check` and `cargo clippy` at minimum.
- If tests exist or were added, run targeted tests first, then `cargo test`.
- Keep diffs focused; avoid opportunistic refactors unless directly relevant.
- Preserve behavior unless task explicitly requests behavioral changes.

## Future Development Notes

- Add focused tests around `ui::action_dispatcher` effect routing (activate/context/dbus/command)
  with injectable hooks or adapters to avoid GTK/runtime coupling.
- Consider throttled telemetry summaries (periodic aggregate logs) under `WAYPIE_DEBUG` for easier
  profiling of redraw/update bursts.
