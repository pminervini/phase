mod app;
mod audio;
mod chord;
mod cli;
mod config;
mod controls;
mod midi;
mod music;
mod stats;
mod trainer;
mod ui;

use anyhow::{Context, Result};
use app::{App, Mode};
use audio::{AudioCommand, AudioEngine, SynthEngine};
use clap::Parser;
use cli::{Cli, Command};
use crossbeam_queue::ArrayQueue;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use midi::{MidiEvent, MidiMessage};
use music::MidiNote;
use ratatui::Terminal;
use ratatui::backend::{CrosstermBackend, TestBackend};
use stats::PracticeStats;
use std::io::{self, IsTerminal};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use trainer::{StaffClef, StaffSong};

const MIDI_QUEUE_CAPACITY: usize = 1024;
const FRAME_TIME: Duration = Duration::from_millis(33);

fn main() -> Result<()> {
    install_panic_hook();
    let cli = Cli::parse();
    let (mut config, config_warning) = config::load()?;

    if cli.smoke_test {
        return smoke_test();
    }

    match cli.command {
        Some(Command::Devices) => print_devices(&cli, &config),
        Some(Command::Monitor { duration }) => monitor(&cli, &config, duration),
        None => {
            if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
                anyhow::bail!(
                    "phase TUI requires an interactive terminal; use --smoke-test for CI"
                );
            }
            let (mut stats, stats_warning) = PracticeStats::load()?;
            let warning = join_warnings(config_warning, stats_warning);
            let started = Instant::now();
            run_tui(&cli, &mut config, &mut stats, warning)?;
            stats.aggregate_practice_seconds = stats
                .aggregate_practice_seconds
                .saturating_add(started.elapsed().as_secs());
            stats.save()?;
            config::save(&config)?;
            Ok(())
        }
    }
}

fn print_devices(cli: &Cli, config: &config::Config) -> Result<()> {
    let midi_devices = midi::inventory()?;
    let requested_midi = cli
        .midi_port
        .as_deref()
        .or(config.preferred_midi_port.as_deref());
    let selected_midi = midi::choose_port(&midi_devices.midi_inputs, requested_midi);
    println!("phase device inventory\n");
    println!("MIDI inputs:");
    if midi_devices.midi_inputs.is_empty() {
        println!("  (none)");
    }
    for (index, name) in midi_devices.midi_inputs.iter().enumerate() {
        println!(
            "  {index}: {name}{}",
            if selected_midi == Some(index) {
                "  [default]"
            } else {
                ""
            }
        );
    }
    println!("\nMIDI outputs:");
    if midi_devices.midi_outputs.is_empty() {
        println!("  (none)");
    }
    for (index, name) in midi_devices.midi_outputs.iter().enumerate() {
        println!("  {index}: {name}");
    }

    let audio_devices = audio::inventory()?;
    let requested_audio = cli
        .audio_device
        .as_deref()
        .or(config.preferred_audio_device.as_deref());
    println!("\nAudio outputs:");
    if audio_devices.outputs.is_empty() {
        println!("  (none)");
    }
    for (index, name) in audio_devices.outputs.iter().enumerate() {
        let selected = requested_audio.map_or_else(
            || audio_devices.default.as_deref() == Some(name.as_str()),
            |needle| name.to_lowercase().contains(&needle.to_lowercase()),
        );
        println!(
            "  {index}: {name}{}",
            if selected { "  [default]" } else { "" }
        );
    }
    println!("\nDefault selection:");
    println!(
        "  MIDI: {}",
        selected_midi
            .and_then(|i| midi_devices.midi_inputs.get(i))
            .map_or("none (use --midi-port)", String::as_str)
    );
    println!(
        "  Audio: {}",
        if cli.no_audio {
            "disabled (--no-audio)"
        } else {
            requested_audio
                .or(audio_devices.default.as_deref())
                .unwrap_or("none")
        }
    );
    Ok(())
}

fn monitor(cli: &Cli, config: &config::Config, duration: Option<u64>) -> Result<()> {
    let queue = Arc::new(ArrayQueue::new(MIDI_QUEUE_CAPACITY));
    let requested = cli
        .midi_port
        .as_deref()
        .or(config.preferred_midi_port.as_deref());
    let Some(connection) = midi::connect(requested, queue.clone(), None, cli.debug)? else {
        println!("phase monitor: no MIDI input devices found");
        return Ok(());
    };
    println!(
        "phase monitor: listening to {}{}",
        connection.name,
        duration.map_or_else(
            || " (Ctrl-C to stop)".into(),
            |seconds| format!(" for {seconds}s")
        )
    );
    let running = Arc::new(AtomicBool::new(true));
    if duration.is_none() {
        let signal = running.clone();
        ctrlc::set_handler(move || signal.store(false, Ordering::Relaxed))
            .context("install Ctrl-C handler")?;
    }
    let started = Instant::now();
    while running.load(Ordering::Relaxed)
        && duration.is_none_or(|seconds| started.elapsed() < Duration::from_secs(seconds))
    {
        while let Some(event) = queue.pop() {
            println!(
                "{:>9.3}  {}",
                started.elapsed().as_secs_f64(),
                event.message
            );
        }
        thread::sleep(Duration::from_millis(4));
    }
    drop(connection);
    println!("phase monitor: stopped");
    Ok(())
}

fn run_tui(
    cli: &Cli,
    config: &mut config::Config,
    stats: &mut PracticeStats,
    warning: Option<String>,
) -> Result<()> {
    let now = Instant::now();
    let mut app = App::new(
        now,
        config.master_volume,
        config.default_bpm,
        (config.training_midi_low, config.training_midi_high),
    );
    app.patch = config.patch;
    app.note_naming = config.note_naming;
    app.warning = warning;
    if let Some(message) = &app.warning {
        app.last_feedback = message.clone();
    }

    let audio = if cli.no_audio {
        None
    } else {
        match AudioEngine::start(
            cli.audio_device
                .as_deref()
                .or(config.preferred_audio_device.as_deref()),
            config.master_volume,
            config.patch,
        ) {
            Ok(engine) => Some(engine),
            Err(error) => {
                app.last_feedback = format!("audio unavailable: {error:#}");
                None
            }
        }
    };
    if let Some(engine) = &audio {
        app.audio_name = engine.device_name.clone();
        app.audio_ok = true;
    } else if cli.no_audio {
        app.audio_name = "disabled".into();
    } else {
        app.audio_name = "unavailable".into();
    }

    let events = Arc::new(ArrayQueue::new(MIDI_QUEUE_CAPACITY));
    let requested_midi = cli
        .midi_port
        .as_deref()
        .or(config.preferred_midi_port.as_deref());
    let midi_connection = match midi::connect(
        requested_midi,
        events.clone(),
        audio.as_ref().map(|engine| engine.queue.clone()),
        cli.debug,
    ) {
        Ok(connection) => connection,
        Err(error) => {
            app.last_feedback = format!("MIDI unavailable: {error:#}");
            None
        }
    };
    if let Some(connection) = &midi_connection {
        app.midi_name = connection.name.clone();
    } else if cli.demo {
        app.midi_name = "demo generator".into();
    }

    let _guard = TerminalSession::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("initialize terminal renderer")?;
    terminal.clear().context("clear terminal")?;

    let result = tui_loop(&mut terminal, &mut app, events, audio.as_ref(), cli.demo);
    if let Some(engine) = &audio {
        engine.send(AudioCommand::AllNotesOff);
    }
    drop(midi_connection);

    config.master_volume = app.volume;
    config.default_bpm = app.bpm;
    config.note_naming = app.note_naming;
    config.patch = app.patch;
    if cli.midi_port.is_some() {
        config.preferred_midi_port.clone_from(&cli.midi_port);
    }
    if cli.audio_device.is_some() {
        config.preferred_audio_device.clone_from(&cli.audio_device);
    }
    record_session(stats, &app);
    result
}

fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    events: Arc<ArrayQueue<MidiEvent>>,
    audio: Option<&AudioEngine>,
    demo: bool,
) -> Result<()> {
    let mut last_frame = Instant::now();
    let mut demo_state = DemoState::new(last_frame);
    loop {
        while let Some(event) = events.pop() {
            app.handle_midi(event);
        }
        let now = Instant::now();
        if demo {
            for event in demo_state.events(
                now,
                app.mode,
                app.staff_exercise.song,
                app.staff_exercise.clef,
            ) {
                if let Some(command) = AudioCommand::from_midi(event.message)
                    && let Some(engine) = audio
                {
                    engine.send(command);
                }
                app.handle_midi(event);
            }
        }
        if app.tick(now)
            && let Some(engine) = audio
        {
            engine.send(AudioCommand::Click { accent: true });
        }
        if let Some(engine) = audio {
            app.audio_ok = engine.is_healthy();
        }
        if now.duration_since(last_frame) >= FRAME_TIME {
            terminal
                .draw(|frame| ui::render(frame, app, now))
                .context("render phase TUI")?;
            last_frame = now;
        }

        if event::poll(Duration::from_millis(5)).context("poll terminal events")?
            && let Event::Key(key) = event::read().context("read terminal event")?
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            if app::ctrl_or_alt(key) {
                continue;
            }
            let old_output = if app.muted { 0.0 } else { app.volume };
            let was_paused = app.paused;
            if app.handle_key(key, now) {
                break;
            }
            let new_output = if app.muted { 0.0 } else { app.volume };
            if (old_output - new_output).abs() > f32::EPSILON
                && let Some(engine) = audio
            {
                engine.send(AudioCommand::SetVolume(new_output));
            }
            if !was_paused
                && app.paused
                && let Some(engine) = audio
            {
                engine.send(AudioCommand::AllNotesOff);
            }
        }
    }
    Ok(())
}

struct DemoState {
    next: Instant,
    step: usize,
    active: Option<MidiNote>,
    mode: Mode,
    staff_selection: (StaffSong, StaffClef),
}

impl DemoState {
    fn new(now: Instant) -> Self {
        Self {
            next: now,
            step: 0,
            active: None,
            mode: Mode::Freeplay,
            staff_selection: (StaffSong::Twinkle, StaffClef::Treble),
        }
    }

    fn events(
        &mut self,
        now: Instant,
        mode: Mode,
        staff_song: StaffSong,
        staff_clef: StaffClef,
    ) -> Vec<MidiEvent> {
        let entering_staff = mode == Mode::Staff && self.mode != Mode::Staff;
        let selection = (staff_song, staff_clef);
        let staff_changed = mode == Mode::Staff && selection != self.staff_selection;
        self.mode = mode;
        self.staff_selection = selection;
        if entering_staff || staff_changed {
            self.step = 0;
            self.next = now;
        }
        if now < self.next {
            return Vec::new();
        }
        self.next = now + Duration::from_millis(430);
        let mut events = Vec::with_capacity(2);
        if let Some(note) = self.active.take() {
            events.push(MidiEvent {
                message: MidiMessage::NoteOff {
                    channel: 0,
                    note,
                    velocity: 0,
                },
                at: now,
            });
        }
        let melody = if mode == Mode::Staff {
            staff_song.treble_notes()
        } else {
            &trainer::TWINKLE_TREBLE
        };
        let transpose = if mode == Mode::Staff && staff_clef == StaffClef::Bass {
            -24
        } else {
            0
        };
        let note = MidiNote::new(melody[self.step % melody.len()])
            .expect("demo note is valid")
            .transpose(transpose)
            .expect("demo clef transposition remains in the MIDI range");
        let velocity = 65 + ((self.step * 17) % 60) as u8;
        events.push(MidiEvent {
            message: MidiMessage::NoteOn {
                channel: 0,
                note,
                velocity,
            },
            at: now,
        });
        self.active = Some(note);
        self.step += 1;
        events
    }
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("enable terminal raw mode")?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide) {
            let _ = disable_raw_mode();
            return Err(error).context("enter terminal session");
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        Show,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original(info);
    }));
}

fn smoke_test() -> Result<()> {
    let now = Instant::now();
    let mut app = App::new(now, 0.72, 100, (48, 72));
    app.midi_name = "smoke-test".into();
    app.audio_name = "offline synth".into();
    app.note_naming = music::NoteNaming::FixedDo;
    let note = MidiNote::new(60).expect("middle C");
    app.handle_midi(MidiEvent {
        message: MidiMessage::NoteOn {
            channel: 0,
            note,
            velocity: 100,
        },
        at: now,
    });

    let mut synth = SynthEngine::new(48_000.0, 0.72);
    synth.process(AudioCommand::NoteOn {
        note,
        velocity: 100,
    });
    for _ in 0..4_800 {
        anyhow::ensure!(
            synth.next_sample().is_finite(),
            "offline synth produced non-finite output"
        );
    }
    synth.process(AudioCommand::NoteOff { note });

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).context("create smoke-test terminal")?;
    terminal
        .draw(|frame| ui::render(frame, &app, now))
        .context("render 80x24 smoke-test frame")?;
    let rendered = format!("{:?}", terminal.backend().buffer());
    anyhow::ensure!(
        rendered.contains("phase") || rendered.contains("PHASE"),
        "TUI smoke frame lacks title"
    );
    anyhow::ensure!(
        rendered.contains("Do4"),
        "TUI smoke frame lacks Fixed Do labels"
    );
    app.mode = app::Mode::Staff;
    terminal
        .draw(|frame| ui::render(frame, &app, now))
        .context("render 80x24 staff smoke-test frame")?;
    let staff_rendered = format!("{:?}", terminal.backend().buffer());
    anyhow::ensure!(
        staff_rendered.contains("STAFF") && staff_rendered.contains('◆'),
        "TUI smoke frame lacks staff or current-note progress marker"
    );
    println!(
        "phase smoke test: ok (MIDI parse/state, offline synth, 80x24 keyboard + staff render)"
    );
    Ok(())
}

fn join_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(a), Some(b)) => Some(format!("{a}; {b}")),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn record_session(stats: &mut PracticeStats, app: &App) {
    let metrics = [
        ("notes", &app.note_exercise.metrics),
        ("staff", &app.staff_exercise.metrics),
        ("scales", &app.scale_exercise.metrics),
        ("rhythm", &app.rhythm_metrics),
    ];
    for (mode, session) in metrics {
        stats.attempts = stats.attempts.saturating_add(session.attempts);
        stats.correct = stats.correct.saturating_add(session.correct);
        let best = stats.best_streaks.entry(mode.into()).or_default();
        *best = (*best).max(session.best_streak);
    }
    for (&note, &count) in &app.note_exercise.weak_notes {
        let total = stats.weak_note_counts.entry(note).or_default();
        *total = total.saturating_add(count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(events: Vec<MidiEvent>) -> MidiNote {
        events
            .into_iter()
            .find_map(|event| match event.message {
                MidiMessage::NoteOn { note, .. } => Some(note),
                _ => None,
            })
            .expect("demo emits a note-on event")
    }

    #[test]
    fn demo_restarts_with_the_selected_staff_song_and_clef() {
        let now = Instant::now();
        let mut demo = DemoState::new(now);
        let freeplay =
            note_on(demo.events(now, Mode::Freeplay, StaffSong::Twinkle, StaffClef::Treble));
        assert_eq!(freeplay.value(), 67);

        let mary = note_on(demo.events(
            now + Duration::from_millis(1),
            Mode::Staff,
            StaffSong::MarysLamb,
            StaffClef::Treble,
        ));
        assert_eq!(mary.value(), 71);

        let bass = note_on(demo.events(
            now + Duration::from_millis(2),
            Mode::Staff,
            StaffSong::MarysLamb,
            StaffClef::Bass,
        ));
        assert_eq!(bass.value(), 47);
    }
}
