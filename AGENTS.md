# AGENTS.md — sd (Stream Deck CLI)

> Guidelines for AI coding agents working in this Rust codebase.

---

## RULE NUMBER 1: NO FILE DELETION

**YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION.** Even a new file that you yourself created, such as a test code file. You have a horrible track record of deleting critically important files or otherwise throwing away tons of expensive work. As a result, you have permanently lost any and all rights to determine that a file or folder should be deleted.

**YOU MUST ALWAYS ASK AND RECEIVE CLEAR, WRITTEN PERMISSION BEFORE EVER DELETING A FILE OR FOLDER OF ANY KIND.**

---

## Irreversible Git & Filesystem Actions — DO NOT EVER BREAK GLASS

1. **Absolutely forbidden commands:** `git reset --hard`, `git clean -fd`, `rm -rf`, or any command that can delete or overwrite code/data must never be run unless the user explicitly provides the exact command and states, in the same message, that they understand and want the irreversible consequences.
2. **No guessing:** If there is any uncertainty about what a command might delete or overwrite, stop immediately and ask the user for specific approval.
3. **Safer alternatives first:** When cleanup or rollbacks are needed, request permission to use non-destructive options (`git status`, `git diff`, `git stash`, copying to backups) before ever considering a destructive command.
4. **Mandatory explicit plan:** Even after explicit user authorization, restate the command verbatim, list exactly what will be affected, and wait for a confirmation that your understanding is correct.

---

## sd (Stream Deck CLI) — This Project

**This is the project you're working on.** `sd` is a cross-platform Rust CLI for controlling Elgato Stream Deck devices. It provides both human-friendly and AI agent-friendly (robot mode) interfaces.

### Architecture

```
CLI (clap) → Device Module (elgato-streamdeck) → HID Communication
     ↓
Robot Mode (JSON output for AI agents)
```

### Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point, command dispatch, robot mode output |
| `src/cli/mod.rs` | Clap argument definitions, command structs |
| `src/device.rs` | Device wrapper around elgato-streamdeck crate |
| `src/error.rs` | Error types with user-recoverable hints |
| `Cargo.toml` | Dependencies and release optimizations |
| `build.rs` | Build script for version metadata (vergen) |

### Robot Mode

The CLI is designed for AI agent ergonomics. Use `--robot` or `--format=json` for machine-parseable output:

```bash
# Quick-start for agents
sd --robot

# List devices as JSON
sd list --robot

# Errors include recovery hints
sd info --robot  # Returns { "error": true, "suggestion": "..." }
```

### Device Commands

| Command | Purpose |
|---------|---------|
| `sd list` | List connected Stream Deck devices |
| `sd info` | Show detailed device information |
| `sd brightness <0-100>` | Set display brightness |
| `sd set-key <key> <image>` | Set a key's image from file |
| `sd fill-key <key> <color>` | Fill key with hex color |
| `sd clear-key <key>` | Clear a single key |
| `sd clear-all` | Clear all keys |
| `sd watch` | Stream button press events |
| `sd read` | Read current button states once |

### Key Layout (Stream Deck XL 32-key)

```
Row 0: [0] [1] [2] [3] [4] [5] [6] [7]
Row 1: [8] [9] [10][11][12][13][14][15]
Row 2: [16][17][18][19][20][21][22][23]
Row 3: [24][25][26][27][28][29][30][31]
```

---

## Toolchain: Rust & Cargo

We only use **Cargo** in this project, NEVER any other package manager.

- **Edition:** Rust 2024 (nightly required — see `rust-toolchain.toml`)
- **Dependency versions:** Explicit versions for stability
- **Configuration:** Cargo.toml only
- **Unsafe code:** Forbidden (`#![forbid(unsafe_code)]`)

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `elgato-streamdeck` | HID communication with Stream Deck devices |
| `image` | Image loading and resizing |
| `serde` + `serde_json` | JSON serialization for robot mode |
| `thiserror` | Error type definitions |
| `colored` | Terminal colors with TTY detection |
| `vergen-gix` | Build metadata embedding |

### Release Profile

The release build optimizes for binary size:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for better optimization
panic = "abort"     # Smaller binary, no unwinding overhead
strip = true        # Remove debug symbols
```

---

## Code Editing Discipline

### No Script-Based Changes

**NEVER** run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve.

- **Always make code changes manually**, even when there are many instances
- For many simple changes: use parallel subagents
- For subtle/complex changes: do them methodically yourself

### No File Proliferation

If you want to change something or add a feature, **revise existing code files in place**.

**NEVER** create variations like:
- `mainV2.rs`
- `main_improved.rs`
- `main_enhanced.rs`

New files are reserved for **genuinely new functionality** that makes zero sense to include in any existing file.

---

## Compiler Checks (CRITICAL)

**After any substantive code changes, you MUST verify no errors were introduced:**

```bash
# Check for compiler errors and warnings
cargo check --all-targets

# Check for clippy lints (pedantic + nursery are enabled)
cargo clippy --all-targets -- -D warnings

# Verify formatting
cargo fmt --check
```

---

## Testing

```bash
# Run all tests
cargo test

# Test with device (requires Stream Deck connected)
cargo run -- list
cargo run -- info
cargo run -- brightness 50
```

---

## Third-Party Library Usage

If you aren't 100% sure how to use a third-party library, **SEARCH ONLINE** to find the latest documentation and best practices.

The `elgato-streamdeck` crate API includes:
- `StreamDeck::connect()` to open a device
- `set_button_image()` + `flush()` to update keys
- `read_input()` to get button events
- `set_brightness()` for display brightness

---

## Multi-Agent Coordination

This project may have multiple agents working on it simultaneously. When you see uncommitted changes you don't recall making:

- **NEVER** stash, revert, or overwrite other agents' work
- Treat unknown changes as if you made them yourself
- Just proceed with your task, incorporating their changes

---

## Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage changes
git commit -m "..."     # Commit with descriptive message
git push                # Push to remote (if configured)
```
