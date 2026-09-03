# Repository Guidelines

## Project Structure & Module Organization

`phase` is a Rust 2024 MIDI instrument and piano tutor. Code lives in `src/`: `main.rs` and `cli.rs` handle startup and commands; `midi.rs` parses and connects MIDI; `audio.rs` owns the real-time synthesizer; `music.rs` and `chord.rs` contain theory logic; `trainer.rs` scores exercises; `app.rs` manages state; `ui.rs` renders Ratatui; and `config.rs`, `stats.rs`, and `controls.rs` handle persistence and controller mappings. Tests are colocated in each module under `#[cfg(test)]`. No external audio assets are required; do not commit `target/`.

## Build, Test, and Development Commands

- `cargo run -- --demo` launches the TUI with synthetic notes.
- `cargo run -- devices` lists MIDI and audio devices and default selections.
- `cargo test` runs the complete unit test suite.
- `cargo fmt --check` verifies standard Rust formatting.
- `cargo clippy --all-targets --all-features -- -D warnings` treats every lint warning as an error.
- `cargo build --release` produces the usable binary at `target/release/phase`.
- `./target/release/phase --smoke-test --no-audio` exercises MIDI parsing, state, synthesis, and an 80x24 render noninteractively.

Run formatting, tests, Clippy, the release build, and the smoke test before submitting changes. Commit updated `Cargo.lock` whenever dependencies change.

## Coding Style & Naming Conventions

Use `rustfmt` defaults (four-space indentation). Follow Rust naming conventions: `snake_case` for functions/modules, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep modules focused and pure logic testable. Add error context with `anyhow` rather than discarding causes. MIDI and audio callbacks are real-time-sensitive: never render, log, allocate, block, perform file I/O, or acquire ordinary mutexes inside them. Keep histories and queues bounded.

## Testing Guidelines

Name tests after behavior, for example `zero_velocity_note_on_is_note_off`. Add regression tests beside the affected module. Audio tests must assert finite, bounded samples and avoid requiring physical hardware. Use the smoke-test path for CI and reserve controller/audio checks for documented manual verification.

## Commit & Pull Request Guidelines

Recent commits use short, imperative, capitalized subjects such as `Fix piano keyboard geometry and labels`. Keep each commit focused. Pull requests should explain user-visible behavior, list exact verification commands, identify hardware checks actually performed, and include an 80x24 screenshot for TUI changes. Link related issues and call out configuration format changes or limitations.

## Configuration & Security

Configuration and practice statistics use platform directories resolved by the `directories` crate. Do not commit local config, credentials, captured MIDI data, or generated artifacts. Preserve malformed-config recovery and version configuration schema changes.
