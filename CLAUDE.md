# CLAUDE.md – fs-session

## What is this?

FreeSynergy Session — runtime context of a logged-in user.
Tracks which user is active, which programs are open, and their window states.
Solves the "minimize problem": restoring a minimized program finds the existing window.

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

- `SessionStore` — open database, create/close sessions, open/close programs
- `Session` — a user session (user id, display name, started_at)
- `ProgramEntry` — an open program window with its state
- `ProgramState` — `Open`, `Minimized`, `Background`

## Dependencies

- `sea-orm =2.0.0-rc.37` (SQLite)
