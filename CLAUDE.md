# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`rc` is a Norton Commander-style dual-panel terminal file manager written in Rust. The goal is a keyboard-driven TUI with two side-by-side directory panels, a command line at the bottom, and function-key shortcuts.

## Build & Run

```bash
cargo build          # debug build
cargo build --release
cargo run
cargo test
cargo test <test_name>   # run a single test
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

The project is in its initial scaffolding stage (`src/main.rs` is a hello-world stub, no dependencies yet).

Planned/expected structure as the project grows:

- **TUI rendering** — will likely use `ratatui` (or `crossterm`/`tui`) for drawing panels, borders, and the status bar
- **Dual-panel model** — two independent `Panel` states (current dir, cursor position, sort order, selection set); one panel is "active" at a time
- **Input loop** — raw-mode keyboard event loop mapping keys/F-keys to commands
- **File operations** — copy, move, delete, mkdir, rename executed on selected files between panels
- **Command bar** — bottom shell-command input line

Add dependencies to `Cargo.toml` as each layer is introduced.
