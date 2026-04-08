# Contributing to Chirp

## Setup

```bash
git clone https://github.com/Chessing234/chirp.git
cd chirp
cargo build
cargo test
```

Requires Rust 1.70+. On Linux, install `libdbus-1-dev` for notifications.

## Before submitting a PR

1. **Tests pass**: `cargo test`
2. **Clippy clean**: `cargo clippy -- -D warnings`
3. **No unnecessary dependencies**: keep the dependency list minimal
4. **Cross-platform**: use `#[cfg(target_os = "...")]` for platform-specific code. Never use Unix-only APIs without a Windows alternative.

## Code conventions

- All DB tests use `Database::in_memory()` to avoid touching real data
- Parser tests are pure functions, no side effects
- Commit messages use imperative mood ("Add feature", not "Added feature")
- Keep PRs focused: one feature or fix per PR

## Architecture

```
src/main.rs     CLI routing, TUI event loop, key/mouse handling
src/app.rs      App state, CRUD logic, undo stack, search
src/ui.rs       Ratatui rendering (all drawing code)
src/db.rs       SQLite schema, migrations, queries
src/parser.rs   Natural language parsing (dates, priorities, pings)
src/daemon.rs   Background reminder service, OS service install
src/import.rs   JSON/CSV bulk import
```

## Adding a new CLI subcommand

1. Add the match arm in `main()` in `main.rs`
2. Add the handler function below `export_tasks()`
3. Update the `--help` text
4. Update README CLI section

## Adding a new keybinding

1. Add the handler in `handle_normal()` in `main.rs`
2. Add the logic in `app.rs`
3. Add to the help overlay in `ui.rs` (`draw_help`)
4. Add to the keybind bar in `ui.rs` (`draw_keybinds`) if it's a primary action

## Release process

```bash
# Bump version in Cargo.toml
git tag v0.X.0
git push --tags
```

GitHub Actions builds binaries for macOS (universal), Linux, and Windows.
