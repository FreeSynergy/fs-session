# fs-session

User session management for FreeSynergy — tracks which user is logged in
and which programs are currently open.

## Build

```sh
cargo build --release
cargo test
```

## Architecture

- `SessionStore` — open database, create/close sessions, open/close programs
- `Session` — a user session (user id, display name, started_at)
- `ProgramEntry` — an open program window with its window state
- `ProgramState` — `Open`, `Minimized`, `Background`

Solves the "minimize problem": minimizing a program marks it as `Minimized`
rather than closing it. Restoring finds the existing window instead of
launching a new instance.
