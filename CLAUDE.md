# CLAUDE.md – fs-session

## What is this?

FreeSynergy Session — runtime context of a logged-in user.
Tracks which user is active, which applications are open, and their window states.
Solves the "minimize problem": restoring a minimized app finds the existing window.

## Rules

- Language in files: **English** (comments, code, variable names)
- Language in chat: **German**
- OOP everywhere: traits over match blocks, types carry their own behavior
- No CHANGELOG.md
- After every feature: commit directly

## Quality Gates (before every commit)

```
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

Every lib.rs / main.rs must have:
```rust
#![deny(clippy::all, clippy::pedantic, warnings)]
```

## Architecture

- `SessionStore` — **trait**: interface for all session storage backends
- `SqliteSessionStore` — concrete SQLite-backed implementation of `SessionStore`
- `SessionTracker` — **trait**: maps Desktop window events to session state
- `StoreBackedTracker` — default tracker backed by any `SessionStore`
- `Session` — a user session (user id, display name, started_at, apps)
- `AppSession` — one open application window with its state
- `AppState` — `Open`, `Minimized`, `Focused`

Consumer code depends on `SessionStore` and `SessionTracker` traits only,
never on `SqliteSessionStore` directly.

## Dependencies

- `async-trait = "0.1"` — trait with async methods
- `sea-orm =2.0.0-rc.37` (SQLite backend for `SqliteSessionStore`)
