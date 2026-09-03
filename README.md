# phase

`phase` is an offline terminal MIDI instrument and interactive piano tutor for macOS. It connects directly to CoreMIDI, synthesizes a velocity-sensitive electric-piano patch through CoreAudio, and renders a focused cyberpunk TUI. It needs no DAW, network service, external synthesizer, or SoundFont.

## Build and run

Rust 1.85 or newer is required for edition 2024. On macOS:

```sh
cargo build --release
./target/release/phase
```

The application prefers a MIDI input containing `MPKmini2` (case-insensitive). If it is absent and exactly one input exists, that input is used. Select devices explicitly when needed:

```sh
./target/release/phase devices
./target/release/phase --midi-port MPKmini2
./target/release/phase --audio-device "MacBook Pro Speakers"
./target/release/phase --demo
./target/release/phase --demo --no-audio
```

Inspect decoded controller traffic without starting the TUI:

```sh
./target/release/phase monitor
./target/release/phase monitor --duration 10
```

CI and noninteractive environments can exercise MIDI parsing/state, offline synthesis, and an 80×24 render:

```sh
./target/release/phase --smoke-test --no-audio
```

## Interface

The four modes are:

- **freeplay** — immediate synthesis, held/sustained-note visualization, raw velocity, and inversion-independent triad/seventh chord detection.
- **notes** — randomized targets in the configured MIDI range, response timing, accuracy, streaks, and weak-note tracking. Press `h` to hide note names.
- **scales** — chromatic, major, natural minor, and major/minor pentatonic practice, ascending and descending, with recent mistakes and completion time.
- **rhythm** — audible/visible beat practice with signed offsets. Grades are perfect at ±35 ms, good at ±80 ms, early/late through ±180 ms, and miss outside that window.

Controls:

| Key | Action |
| --- | --- |
| `q` | Quit |
| `Tab` / `Shift-Tab` | Next / previous mode |
| `Space` | Pause / resume |
| `r` | Restart current exercise |
| `←` / `→` | Change scale root or exercise option |
| `↑` / `↓` | Change scale or difficulty |
| `+` / `-` | Adjust BPM |
| `m` | Mute |
| `[` / `]` | Adjust master volume |
| `?` | Toggle help overlay |

Terminal keys never generate piano notes; musical input comes only from MIDI or demo mode.

## 80×24 mockup

```text
 phase  offline midi instrument + tutor                         PHASE // FREEPLAY
┌ SYSTEM ─────────────────────────────────────────────────────────────────────┐
│MIDI MPKmini2  AUDIO Speakers (online)  100 BPM  SUS off  VOL 72%  00:42    │
└─────────────────────────────────────────────────────────────────────────────┘
┌ TARGET ─────────────────────────────────────────────────────────────────────┐
│  C major      Play freely · chord detection ignores inversions             │
└─────────────────────────────────────────────────────────────────────────────┘
┌ KEYBOARD · C3—C5 ───────────────────────────────────────────────────────────┐
│   ███   ███      ███   ███   ███      ███   ███      ███   ███   ███      │
│ │  ║  │  ║  │  │  ║  │  ║  │  ║  │  │  ║  │  ║  │  │  ║  │  ║  │  ║  │  │
│ C C#  D D#  E  F F#  G G#  A A#  B  C C#  D D#  E  F F#  G G#  A A#  B  C│
│active: C4 [ 60] v104  E4 [ 64] v 91  G4 [ 67] v 96                      │
└─────────────────────────────────────────────────────────────────────────────┘
┌ RECENT MIDI ──────────────────────────┐┌ SESSION ───────────────────────────┐
│ch 1 note on   67  G4 vel  96         ││detected chord  C major            │
│ch 1 note on   64  E4 vel  91         ││velocity-sensitive 32-voice piano  │
│ch 1 note on   60  C4 vel 104         ││                                   │
└───────────────────────────────────────┘└────────────────────────────────────┘
 q quit  tab mode  space pause  r restart  +/- bpm  m mute  [/] volume  ? help
```

## Architecture

- `main.rs` / `cli.rs` — startup, subcommands, terminal lifecycle, demo and smoke paths.
- `midi.rs` — pure defensive MIDI decoding, CoreMIDI discovery/selection, bounded callback handoff.
- `audio.rs` — CPAL/CoreAudio stream and allocation-free 32-voice synthesizer. The callback drains a fixed-capacity nonblocking command queue and performs no file I/O, logging, locks, or allocation.
- `music.rs` / `chord.rs` — MIDI note, pitch-class and scale primitives plus inversion-independent chord matching.
- `trainer.rs` — exercise state, centralized score thresholds and monotonic timing.
- `app.rs` — bounded application state and event transitions, separate from callbacks and rendering.
- `ui.rs` — approximately 30 FPS Ratatui renderer, including the 80×24 compact layout.
- `config.rs` / `stats.rs` — versioned TOML configuration and aggregate practice data.

The MIDI callback sends compact note/pedal commands directly to the bounded audio queue for low latency and separately queues decoded events for the TUI. Queue overflow drops the newest event rather than blocking a real-time-sensitive thread. Synthesis uses sample-rate-independent envelopes, deterministic oldest-voice stealing, smooth release, and finite soft-clipped output.

## Configuration and data

The `directories` crate resolves platform paths for the `phase` application. On macOS these are under the user Library directories. Run `phase` once to create the human-readable versioned files:

- configuration: the project configuration directory, `phase/config.toml`
- practice totals: the project local data directory, `phase/practice.toml`

Configuration stores preferred device substrings, volume, BPM, training range, and theme. Practice data stores total time, accuracy totals, weak-note counts, and best streaks. A malformed file is nonfatal: `phase` uses safe defaults and displays a warning.

## Troubleshooting

- Run `phase devices` first. If several MIDI inputs exist and none is `MPKmini2`, pass `--midi-port <substring>`.
- Use `phase monitor --duration 10` to confirm CoreMIDI events. Note-on velocity zero is decoded as note-off.
- If audio cannot open, inspect the listed output name, pass `--audio-device <substring>`, or use `--no-audio` while diagnosing it.
- If the terminal is too small, enlarge it to at least 80×24. The panic hook and terminal guard restore raw mode, cursor visibility, mouse capture, and the original screen on exit.
- Demo mode works without hardware: `phase --demo`.

## Development checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
./target/release/phase --smoke-test --no-audio
```

Licensed under the MIT License.
