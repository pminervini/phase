use crate::app::{App, Mode};
use crate::music::{MidiNote, NoteNaming};
use crate::trainer::{STAFF_LINE_LENGTH, STAFF_SONG_MENU_COLUMNS, StaffClef, StaffSong};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::time::Instant;

const BG: Color = Color::Rgb(7, 9, 15);
const PANEL: Color = Color::Rgb(14, 18, 29);
const CYAN: Color = Color::Rgb(51, 222, 255);
const MAGENTA: Color = Color::Rgb(255, 67, 200);
const VIOLET: Color = Color::Rgb(156, 105, 255);
const DIM: Color = Color::Rgb(98, 105, 122);
const TEXT: Color = Color::Rgb(214, 221, 235);

pub fn render(frame: &mut Frame<'_>, app: &App, now: Instant) {
    frame.render_widget(
        Block::default().style(Style::default().bg(BG)),
        frame.area(),
    );
    let area = frame.area();
    let compact = area.height <= 24;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if compact && app.mode == Mode::Staff {
            [
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Length(7),
                Constraint::Min(6),
                Constraint::Length(1),
            ]
        } else if compact {
            [
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Min(5),
                Constraint::Length(1),
            ]
        } else {
            [
                Constraint::Length(2),
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Length(9),
                Constraint::Min(7),
                Constraint::Length(1),
            ]
        })
        .split(area);

    render_title(frame, app, chunks[0]);
    render_status(frame, app, now, chunks[1]);
    if app.mode == Mode::Staff {
        render_staff(
            frame,
            app,
            Rect {
                x: chunks[2].x,
                y: chunks[2].y,
                width: chunks[2].width,
                height: chunks[2].height + chunks[3].height,
            },
        );
    } else {
        render_exercise(frame, app, chunks[2]);
        render_keyboard(frame, app, chunks[3]);
    }
    render_lower(frame, app, chunks[4]);
    render_footer(frame, app, chunks[5]);
    if app.help {
        render_help(frame, area);
    } else if app.song_menu {
        render_song_menu(frame, app, area);
    }
}

fn cyber_block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            format!(" {} ", title.into()),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(41, 58, 78)))
        .style(Style::default().bg(PANEL).fg(TEXT))
}

fn render_title(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mode = format!(" PHASE // {} ", app.mode.to_string().to_uppercase());
    let line = Line::from(vec![
        Span::styled(
            " phase",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  offline midi instrument + tutor",
            Style::default().fg(DIM),
        ),
        Span::styled(
            mode,
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(line).style(Style::default().bg(BG)), area);
}

fn render_status(frame: &mut Frame<'_>, app: &App, now: Instant, area: Rect) {
    let elapsed = app.session_duration(now).as_secs();
    let audio = if app.audio_ok { "on" } else { "off" };
    let midi_name = compact_name(&app.midi_name, 12);
    let audio_name = compact_name(&app.audio_name, 16);
    let status = Line::from(vec![
        Span::styled("MIDI ", Style::default().fg(DIM)),
        Span::styled(midi_name, Style::default().fg(CYAN)),
        Span::raw(" "),
        Span::styled("AUDIO ", Style::default().fg(DIM)),
        Span::styled(
            format!("{audio_name}:{audio}"),
            Style::default().fg(if app.audio_ok { CYAN } else { MAGENTA }),
        ),
        Span::raw(" "),
        Span::styled(format!("K8 {} BPM", app.bpm), Style::default().fg(VIOLET)),
        Span::raw(" "),
        Span::styled(
            format!("SUS {}", if app.sustain { "ON" } else { "off" }),
            Style::default().fg(if app.sustain { MAGENTA } else { DIM }),
        ),
        Span::raw(" "),
        Span::styled(
            format!("K1 {:>3}%", (app.volume * 100.0).round()),
            Style::default().fg(CYAN),
        ),
        Span::raw(" "),
        Span::styled(
            format!("{:02}:{:02}", elapsed / 60, elapsed % 60),
            Style::default().fg(DIM),
        ),
    ]);
    let patch = Line::from(vec![
        Span::styled("K2 atk ", Style::default().fg(DIM)),
        Span::styled(
            crate::controls::format_time(app.patch.attack_seconds),
            Style::default().fg(CYAN),
        ),
        Span::styled(" K3 dec ", Style::default().fg(DIM)),
        Span::styled(
            crate::controls::format_time(app.patch.decay_seconds),
            Style::default().fg(CYAN),
        ),
        Span::styled(" K4 sus ", Style::default().fg(DIM)),
        Span::styled(
            format!("{:.0}%", app.patch.sustain_level * 100.0),
            Style::default().fg(VIOLET),
        ),
        Span::styled(" K5 rel ", Style::default().fg(DIM)),
        Span::styled(
            crate::controls::format_time(app.patch.release_seconds),
            Style::default().fg(CYAN),
        ),
        Span::styled(" K6 bri ", Style::default().fg(DIM)),
        Span::styled(
            format!("{:.0}%", app.patch.brightness * 100.0),
            Style::default().fg(MAGENTA),
        ),
        Span::styled(" K7 mix ", Style::default().fg(DIM)),
        Span::styled(
            format!("{:.0}%", app.patch.harmonic_mix * 100.0),
            Style::default().fg(MAGENTA),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(vec![status, patch]).block(cyber_block("SYSTEM")),
        area,
    );
}

fn compact_name(value: &str, maximum: usize) -> String {
    let count = value.chars().count();
    if count <= maximum {
        value.into()
    } else {
        value
            .chars()
            .take(maximum.saturating_sub(1))
            .chain(std::iter::once('…'))
            .collect()
    }
}

fn render_exercise(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (target, detail) = match app.mode {
        Mode::Freeplay => (
            app.chord
                .map_or_else(|| "—".into(), |chord| format_chord(app, chord)),
            "Play freely · chord detection ignores inversions".into(),
        ),
        Mode::Notes => {
            let target = if app.note_exercise.hide_name {
                "●".into()
            } else {
                app.note_naming.format_note(app.note_exercise.target)
            };
            let weak = app
                .note_exercise
                .weak_notes
                .iter()
                .max_by_key(|(_, count)| *count)
                .and_then(|(&value, &count)| {
                    crate::music::MidiNote::new(value).map(|note| {
                        format!(" · weak {} ×{count}", app.note_naming.format_note(note))
                    })
                })
                .unwrap_or_default();
            (
                target,
                format!(
                    "MIDI {} · h toggles note-name visibility{weak}",
                    app.note_exercise.target.value(),
                ),
            )
        }
        Mode::Staff => (
            "sight reading".into(),
            "Follow the current note on the staff".into(),
        ),
        Mode::Scales => {
            let mistakes = app
                .scale_exercise
                .recent_mistakes
                .iter()
                .map(|note| app.note_naming.format_note(*note))
                .collect::<Vec<_>>()
                .join(",");
            let completion = app.scale_exercise.last_completion.map_or_else(
                || "—".into(),
                |elapsed| format!("{:.1}s", elapsed.as_secs_f32()),
            );
            (
                app.note_naming.format_note(app.scale_exercise.expected()),
                format!(
                    "{} {} · step {}/{} · {} complete · last {completion} · mistakes {}",
                    app.note_naming.format_pitch_class(app.scale_exercise.root),
                    app.scale_exercise.kind,
                    app.scale_exercise.index + 1,
                    app.scale_exercise.sequence.len(),
                    app.scale_exercise.completed,
                    if mistakes.is_empty() {
                        "—"
                    } else {
                        &mistakes
                    }
                ),
            )
        }
        Mode::Rhythm => (
            app.note_naming.format_note(app.rhythm_note),
            format!(
                "Play on each beat · perfect ±35 ms · good ±80 ms · window ±180 ms · {}",
                app.last_feedback
            ),
        ),
    };
    let line = Line::from(vec![
        Span::styled(
            format!("  {target:<12}"),
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ),
        Span::styled(detail, Style::default().fg(TEXT)),
    ]);
    frame.render_widget(Paragraph::new(line).block(cyber_block("TARGET")), area);
}

fn render_staff(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let exercise = &app.staff_exercise;
    let line_count = exercise.sequence.len().div_ceil(STAFF_LINE_LENGTH);
    let current_line = exercise
        .index
        .min(exercise.sequence.len().saturating_sub(1))
        / STAFF_LINE_LENGTH;
    let progress = (0..line_count)
        .map(|line| {
            if line < current_line || exercise.index == exercise.sequence.len() {
                '■'
            } else if line == current_line {
                '◆'
            } else {
                '·'
            }
        })
        .collect::<String>();
    let status = if exercise.index == exercise.sequence.len() {
        "COMPLETE".into()
    } else {
        format!("NOTE {}/{}", exercise.index + 1, exercise.sequence.len())
    };
    let title = format!(
        "STAFF · {} · {} · {progress} · LINE {}/{} · {status}",
        exercise.song.label().to_uppercase(),
        exercise.clef.label().to_uppercase(),
        current_line + 1,
        line_count
    );
    let block = cyber_block(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let staff_height = inner.height.min(10);
    let staff_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(staff_height) / 2,
        width: inner.width,
        height: staff_height,
    };
    frame.render_widget(
        Paragraph::new(staff_lines(app, usize::from(inner.width))),
        staff_area,
    );
}

fn staff_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let exercise = &app.staff_exercise;
    let mut rows = vec![
        vec![
            PianoCell {
                symbol: ' ',
                style: Style::default().bg(PANEL),
            };
            width
        ];
        10
    ];
    for row in (0..9).step_by(2) {
        for cell in &mut rows[row] {
            cell.symbol = '─';
            cell.style = Style::default().fg(DIM).bg(PANEL);
        }
    }
    if width > 3 {
        let clef = match exercise.clef {
            StaffClef::Treble => 'G',
            StaffClef::Bass => 'F',
        };
        rows[4][1] = PianoCell {
            symbol: clef,
            style: Style::default()
                .fg(VIOLET)
                .bg(PANEL)
                .add_modifier(Modifier::BOLD),
        };
        for row in rows.iter_mut().take(9) {
            row[3].symbol = '│';
            row[3].style = Style::default().fg(DIM).bg(PANEL);
            row[width - 1].symbol = '│';
            row[width - 1].style = Style::default().fg(DIM).bg(PANEL);
        }
    }

    let first_x = 8.min(width.saturating_sub(1));
    let last_x = width.saturating_sub(5).max(first_x);
    let current_line = exercise
        .index
        .min(exercise.sequence.len().saturating_sub(1))
        / STAFF_LINE_LENGTH;
    let start = current_line * STAFF_LINE_LENGTH;
    let end = (start + STAFF_LINE_LENGTH).min(exercise.sequence.len());
    let visible = &exercise.sequence[start..end];
    let denominator = visible.len().saturating_sub(1).max(1);
    for (local_index, note) in visible.iter().copied().enumerate() {
        let index = start + local_index;
        let Some(row) = staff_row(note, exercise.clef) else {
            continue;
        };
        let x = first_x + local_index * last_x.saturating_sub(first_x) / denominator;
        if x >= width {
            continue;
        }
        let (symbol, style) = if index < exercise.index {
            (
                '●',
                Style::default()
                    .fg(CYAN)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            )
        } else if index == exercise.index {
            (
                '◆',
                Style::default()
                    .fg(MAGENTA)
                    .bg(PANEL)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ('●', Style::default().fg(TEXT).bg(PANEL))
        };
        rows[row][x] = PianoCell { symbol, style };
        if is_sharp(note) && x >= 2 {
            rows[row][x - 2] = PianoCell { symbol: '#', style };
        }
        write_centered_at(&mut rows[9], x, &app.note_naming.format_note(note), style);
    }
    rows.into_iter().map(cells_to_line).collect()
}

fn write_centered_at(row: &mut [PianoCell], center: usize, label: &str, style: Style) {
    let label_width = label.chars().count().min(row.len());
    let start = center
        .saturating_sub(label_width / 2)
        .min(row.len().saturating_sub(label_width));
    for (cell, symbol) in row[start..].iter_mut().zip(label.chars()) {
        cell.symbol = symbol;
        cell.style = style;
    }
}

fn staff_row(note: MidiNote, clef: StaffClef) -> Option<usize> {
    let degree = match note.pitch_class().value() {
        0 | 1 => 0,
        2 | 3 => 1,
        4 => 2,
        5 | 6 => 3,
        7 | 8 => 4,
        9 | 10 => 5,
        11 => 6,
        _ => unreachable!("pitch classes are modulo 12"),
    };
    let note_position = i16::from(note.octave()) * 7 + degree;
    let top_position = match clef {
        StaffClef::Treble => 5 * 7 + 3, // F5
        StaffClef::Bass => 3 * 7 + 5,   // A3
    };
    usize::try_from(top_position - note_position)
        .ok()
        .filter(|row| *row < 9)
}

fn is_sharp(note: MidiNote) -> bool {
    matches!(note.pitch_class().value(), 1 | 3 | 6 | 8 | 10)
}

fn format_chord(app: &App, chord: crate::chord::Chord) -> String {
    format!(
        "{} {}",
        app.note_naming.format_pitch_class(chord.root),
        chord.quality
    )
}

fn render_keyboard(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let inner_width = usize::from(area.width.saturating_sub(2));
    let mut lines = piano_lines(app, inner_width);
    let active: Vec<_> = app
        .active_notes()
        .map(|(note, state)| {
            format!(
                "{} [{:>3}] v{:>3}",
                app.note_naming.format_note(note),
                note.value(),
                state.velocity
            )
        })
        .collect();
    lines.push(Line::from(Span::styled(
        if active.is_empty() {
            "active: —".into()
        } else {
            format!("active: {}", active.join("  "))
        },
        Style::default().fg(CYAN),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .block(cyber_block(format!(
                "KEYBOARD · {}—{}",
                app.note_naming.format_note(
                    crate::music::MidiNote::new(app.keyboard_base)
                        .expect("keyboard base is a valid MIDI note")
                ),
                app.note_naming.format_note(
                    crate::music::MidiNote::new(app.keyboard_high())
                        .expect("keyboard high note is valid")
                )
            )))
            .wrap(Wrap { trim: false }),
        area,
    );
}

const WHITE_KEY_OFFSETS: [u8; 15] = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24];
const BLACK_KEY_OFFSETS: [(u8, usize); 10] = [
    (1, 1),
    (3, 2),
    (6, 4),
    (8, 5),
    (10, 6),
    (13, 8),
    (15, 9),
    (18, 11),
    (20, 12),
    (22, 13),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KeyPlacement {
    note: u8,
    start: usize,
    end: usize,
}

#[derive(Clone)]
struct PianoCell {
    symbol: char,
    style: Style,
}

impl Default for PianoCell {
    fn default() -> Self {
        Self {
            symbol: ' ',
            style: Style::default(),
        }
    }
}

fn piano_geometry(width: usize, base: u8) -> (Vec<KeyPlacement>, Vec<KeyPlacement>) {
    if width == 0 {
        return (Vec::new(), Vec::new());
    }
    let white = WHITE_KEY_OFFSETS
        .iter()
        .enumerate()
        .map(|(index, &offset)| KeyPlacement {
            note: base + offset,
            start: index * width / WHITE_KEY_OFFSETS.len(),
            end: (index + 1) * width / WHITE_KEY_OFFSETS.len(),
        })
        .collect::<Vec<_>>();
    let black_width = if width >= 45 { 3 } else { 1 };
    let black = BLACK_KEY_OFFSETS
        .iter()
        .map(|&(offset, boundary)| {
            let center = boundary * width / WHITE_KEY_OFFSETS.len();
            let start = center
                .saturating_sub(black_width / 2)
                .min(width.saturating_sub(black_width));
            KeyPlacement {
                note: base + offset,
                start,
                end: (start + black_width).min(width),
            }
        })
        .collect();
    (white, black)
}

fn piano_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let (white_keys, black_keys) = piano_geometry(width, app.keyboard_base);
    let mut rows = vec![vec![PianoCell::default(); width]; 4];

    for key in &white_keys {
        let style = piano_key_style(app, key.note, false);
        let label = app.note_naming.format_note(
            crate::music::MidiNote::new(key.note).expect("white-key MIDI note is valid"),
        );
        for row in &mut rows {
            for cell in &mut row[key.start..key.end] {
                cell.style = style;
            }
        }
        for row in &mut rows[2..] {
            if let Some(cell) = row.get_mut(key.start) {
                cell.symbol = '│';
            }
        }
        let label_area = KeyPlacement {
            start: (key.start + 1).min(key.end),
            ..*key
        };
        write_centered(&mut rows[3], label_area, &label, style);
    }
    if width > 0 {
        rows[2][width - 1].symbol = '│';
        rows[3][width - 1].symbol = '│';
    }

    for key in &black_keys {
        let style = piano_key_style(app, key.note, true);
        let note = crate::music::MidiNote::new(key.note).expect("black-key MIDI note is valid");
        for row in &mut rows[..2] {
            for cell in &mut row[key.start..key.end] {
                cell.symbol = ' ';
                cell.style = style;
            }
        }
        if app.note_naming == NoteNaming::FixedDo {
            let syllable = app
                .note_naming
                .format_pitch_class(note.pitch_class())
                .trim_end_matches('#');
            let accidental_octave = format!("#{}", note.octave());
            write_centered(&mut rows[0], *key, syllable, style);
            write_centered(&mut rows[1], *key, &accidental_octave, style);
        } else {
            write_centered(
                &mut rows[1],
                *key,
                &app.note_naming.format_note(note),
                style,
            );
        }
    }

    rows.into_iter().map(cells_to_line).collect()
}

fn piano_key_style(app: &App, note: u8, black: bool) -> Style {
    let state = app.notes[usize::from(note)];
    if state.velocity > 0 {
        let brightness = 96_u8.saturating_add(state.velocity);
        let background = if black {
            Color::Rgb(brightness, 38, 196)
        } else {
            Color::Rgb(18, brightness, 224)
        };
        Style::default()
            .fg(Color::Black)
            .bg(background)
            .add_modifier(Modifier::BOLD)
    } else if black {
        Style::default()
            .fg(Color::Rgb(134, 143, 164))
            .bg(Color::Rgb(20, 23, 35))
    } else {
        Style::default()
            .fg(Color::Rgb(193, 201, 216))
            .bg(Color::Rgb(53, 58, 71))
    }
}

fn write_centered(row: &mut [PianoCell], key: KeyPlacement, label: &str, style: Style) {
    let available = key.end.saturating_sub(key.start);
    let label_width = label.chars().count().min(available);
    let offset = key.start + available.saturating_sub(label_width) / 2;
    for (cell, symbol) in row[offset..key.end]
        .iter_mut()
        .zip(label.chars().take(label_width))
    {
        cell.symbol = symbol;
        cell.style = style;
    }
}

fn cells_to_line(cells: Vec<PianoCell>) -> Line<'static> {
    let mut spans = Vec::new();
    let mut symbols = String::new();
    let mut style = cells.first().map_or_else(Style::default, |cell| cell.style);
    for cell in cells {
        if cell.style != style && !symbols.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut symbols), style));
            style = cell.style;
        }
        symbols.push(cell.symbol);
    }
    if !symbols.is_empty() {
        spans.push(Span::styled(symbols, style));
    }
    Line::from(spans)
}

fn render_lower(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(53), Constraint::Percentage(47)])
        .split(area);
    let recent: Vec<Line<'_>> = app
        .recent
        .iter()
        .rev()
        .take(5)
        .map(|event| {
            Line::from(Span::styled(
                app.format_midi_message(*event),
                Style::default().fg(DIM),
            ))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(recent).block(cyber_block("RECENT MIDI")),
        columns[0],
    );

    let metrics = app.metrics();
    let text = if let Some(metrics) = metrics {
        let timing = if app.mode == Mode::Rhythm {
            metrics.mean_timing_error_ms().map_or_else(
                || "mean |timing| —".into(),
                |value| format!("mean |timing| {value:.1} ms"),
            )
        } else {
            metrics.mean_response_time_ms().map_or_else(
                || "mean response —".into(),
                |value| format!("mean response {value:.0} ms"),
            )
        };
        vec![
            Line::from(vec![
                Span::styled(
                    format!("accuracy {:>5.1}%", metrics.accuracy()),
                    Style::default().fg(CYAN),
                ),
                Span::raw("   "),
                Span::styled(
                    format!("streak {} / best {}", metrics.streak, metrics.best_streak),
                    Style::default().fg(VIOLET),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("attempts {}", metrics.attempts),
                    Style::default().fg(DIM),
                ),
                Span::raw("   "),
                Span::styled(timing, Style::default().fg(MAGENTA)),
            ]),
            Line::from(Span::styled(&app.last_feedback, Style::default().fg(TEXT))),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                format!(
                    "detected chord  {}",
                    app.chord
                        .map_or_else(|| "—".into(), |chord| format_chord(app, chord))
                ),
                Style::default().fg(MAGENTA),
            )),
            Line::from(Span::styled(
                "velocity-sensitive 32-voice electric piano",
                Style::default().fg(DIM),
            )),
            Line::from(Span::styled(
                app.last_control
                    .as_deref()
                    .map_or("knobs K1—K8 ready", |control| control),
                Style::default().fg(CYAN),
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(text).block(cyber_block("SESSION")),
        columns[1],
    );
}

const FOOTER: &str = " q quit  tab mode  n names  space pause  r reset  +/- bpm  m mute  ? help ";
const STAFF_FOOTER: &str =
    " q quit  tab mode  enter/s songs  ↑/↓ clef  n names  pause  reset  ? help ";

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(if app.mode == Mode::Staff {
            STAFF_FOOTER
        } else {
            FOOTER
        })
        .style(Style::default().fg(DIM).bg(BG))
        .alignment(Alignment::Center),
        area,
    );
}

fn render_song_menu(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let popup = centered_fixed(78, 12, area);
    frame.render_widget(Clear, popup);
    let mut lines = vec![Line::from(Span::styled(
        "Choose a song for staff practice  • current",
        Style::default().fg(TEXT),
    ))];
    let row_count = StaffSong::ALL.len().div_ceil(STAFF_SONG_MENU_COLUMNS);
    let inner_width = usize::from(popup.width.saturating_sub(2));
    let column_width = inner_width / STAFF_SONG_MENU_COLUMNS;
    for row in 0..row_count {
        let mut spans = Vec::with_capacity(STAFF_SONG_MENU_COLUMNS);
        for column in 0..STAFF_SONG_MENU_COLUMNS {
            let index = column * row_count + row;
            let width = if column + 1 == STAFF_SONG_MENU_COLUMNS {
                inner_width - column_width * column
            } else {
                column_width
            };
            let Some(song) = StaffSong::ALL.get(index) else {
                spans.push(Span::raw(" ".repeat(width)));
                continue;
            };
            let highlighted = index == app.song_menu_index;
            let marker = if highlighted { "▶ " } else { "  " };
            let current = if *song == app.staff_exercise.song {
                " •"
            } else {
                ""
            };
            spans.push(Span::styled(
                pad_text(&format!("{marker}{}{current}", song.label()), width),
                Style::default()
                    .fg(if highlighted {
                        MAGENTA
                    } else if *song == app.staff_exercise.song {
                        CYAN
                    } else {
                        TEXT
                    })
                    .add_modifier(if highlighted {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Arrow keys move   Enter select   Esc close",
        Style::default().fg(DIM),
    )));
    frame.render_widget(
        Paragraph::new(lines).block(cyber_block("SONG LIBRARY")),
        popup,
    );
}

fn pad_text(value: &str, width: usize) -> String {
    let mut value = compact_name(value, width);
    value.push_str(&" ".repeat(width.saturating_sub(value.chars().count())));
    value
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(90, 84, area);
    frame.render_widget(Clear, popup);
    let help = vec![
        Line::from(Span::styled(
            "PHASE CONTROL MATRIX",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("q quit      tab / shift-tab change mode      space pause/resume"),
        Line::from("r restart   enter/s staff song menu   ↑/↓ scale or staff clef"),
        Line::from("+/- BPM     m mute              [/] master volume"),
        Line::from("n toggle note names: Letters / Fixed Do"),
        Line::from("h hide/show target note name (notes mode)    ? or esc close"),
        Line::from(""),
        Line::from(Span::styled(
            "MPK knobs: K1 volume · K2 attack · K3 decay · K4 sustain",
            Style::default().fg(VIOLET),
        )),
        Line::from(Span::styled(
            "           K5 release · K6 brightness · K7 harmonics · K8 BPM",
            Style::default().fg(VIOLET),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "MIDI keyboard input is independent of terminal keys.",
            Style::default().fg(CYAN),
        )),
        Line::from("Staff: enter/s song menu · ◆ current · cyan notes complete."),
        Line::from("Rhythm: perfect ±35 ms, good ±80 ms, early/late to ±180 ms."),
    ];
    frame.render_widget(
        Paragraph::new(help)
            .block(cyber_block("HELP"))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NoteState;
    use std::time::Instant;

    fn row_text(row: &[PianoCell]) -> String {
        row.iter().map(|cell| cell.symbol).collect()
    }

    #[test]
    fn piano_geometry_centers_black_keys_on_white_boundaries() {
        let (white, black) = piano_geometry(78, 48);
        assert_eq!(white.len(), 15);
        assert_eq!(black.len(), 10);
        assert_eq!(white.first().unwrap().note, 48);
        assert_eq!(white.last().unwrap().note, 72);
        assert_eq!(white.last().unwrap().end, 78);

        for key in black {
            let boundary = BLACK_KEY_OFFSETS
                .iter()
                .find(|(offset, _)| 48 + *offset == key.note)
                .map(|(_, boundary)| *boundary)
                .unwrap();
            assert_eq!((key.start + key.end) / 2, white[boundary].start);
        }
    }

    #[test]
    fn piano_labels_are_anchored_inside_their_keys() {
        let now = Instant::now();
        let app = App::new(now, 0.7, 100, (48, 72));
        let (white, black) = piano_geometry(78, 48);
        let mut rows = vec![vec![PianoCell::default(); 78]; 4];
        for key in &white {
            write_centered(
                &mut rows[3],
                *key,
                &crate::music::MidiNote::new(key.note).unwrap().to_string(),
                Style::default(),
            );
        }
        for key in &black {
            write_centered(
                &mut rows[1],
                *key,
                &crate::music::MidiNote::new(key.note).unwrap().to_string(),
                Style::default(),
            );
        }
        let black_labels = row_text(&rows[1]);
        let white_labels = row_text(&rows[3]);
        assert_eq!(&black_labels[black[0].start..black[0].end], "C#3");
        assert!(white_labels[white[0].start..white[0].end].contains("C3"));
        assert!(white_labels[white[7].start..white[7].end].contains("C4"));
        assert!(white_labels[white[14].start..white[14].end].contains("C5"));

        let lines = piano_lines(&app, 78);
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn piano_geometry_transposes_with_keyboard_window() {
        let (white, black) = piano_geometry(78, 60);
        assert_eq!(white.first().unwrap().note, 60);
        assert_eq!(white.last().unwrap().note, 84);
        assert_eq!(black.first().unwrap().note, 61);
        assert_eq!(black.last().unwrap().note, 82);
    }

    #[test]
    fn fixed_do_piano_labels_fit_at_eighty_columns() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.note_naming = NoteNaming::FixedDo;
        let lines = piano_lines(&app, 78);
        let black_syllables = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let black_accidentals = lines[1]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let white_labels = lines[3]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(black_syllables.contains("Sol"));
        assert!(black_accidentals.contains("#3"));
        assert!(white_labels.contains("Do3"));
        assert!(white_labels.contains("Si4"));
        assert_eq!(
            white_labels.chars().filter(|symbol| *symbol == '│').count(),
            16
        );

        let (_, black_keys) = piano_geometry(78, 48);
        assert!(
            black_keys
                .windows(2)
                .all(|keys| keys[0].end < keys[1].start)
        );
    }

    #[test]
    fn active_key_style_changes_only_the_matching_note() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        let inactive = piano_key_style(&app, 60, false);
        app.notes[60] = NoteState {
            velocity: 100,
            held: true,
        };
        assert_ne!(piano_key_style(&app, 60, false), inactive);
        assert_eq!(piano_key_style(&app, 62, false), inactive);
    }

    #[test]
    fn staff_places_notes_by_clef_and_marks_current_progress() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.mode = Mode::Staff;
        app.staff_exercise.sequence = vec![
            MidiNote::new(64).unwrap(),
            MidiNote::new(66).unwrap(),
            MidiNote::new(77).unwrap(),
        ];
        app.staff_exercise.index = 1;

        assert_eq!(
            staff_row(MidiNote::new(64).unwrap(), StaffClef::Treble),
            Some(8)
        );
        assert_eq!(
            staff_row(MidiNote::new(77).unwrap(), StaffClef::Treble),
            Some(0)
        );
        let text = staff_lines(&app, 78)
            .into_iter()
            .flat_map(|line| line.spans)
            .map(|span| span.content.into_owned())
            .collect::<String>();
        assert!(text.contains('◆'));
        assert!(text.contains('#'));
        assert!(text.contains("E4"));
        assert!(text.contains("F#4"));

        app.note_naming = NoteNaming::FixedDo;
        let fixed_do = staff_lines(&app, 78)
            .into_iter()
            .flat_map(|line| line.spans)
            .map(|span| span.content.into_owned())
            .collect::<String>();
        assert!(fixed_do.contains("Mi4"));
        assert!(fixed_do.contains("Fa#4"));
    }

    #[test]
    fn staff_mode_renders_progress_at_eighty_columns() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.mode = Mode::Staff;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app, now)).unwrap();
        let rendered = format!("{:?}", terminal.backend().buffer());
        assert!(rendered.contains("STAFF"));
        assert!(rendered.contains("TREBLE"));
        assert!(rendered.contains('◆'));
        assert!(rendered.contains("TWINKLE"));
        assert!(rendered.contains("NOTE 1/42"));
        assert!(rendered.contains("G4"));
    }

    #[test]
    fn song_menu_lists_every_choice_at_eighty_columns() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.mode = Mode::Staff;
        app.song_menu = true;
        app.song_menu_index = 1;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app, now)).unwrap();
        let rendered = format!("{:?}", terminal.backend().buffer());
        assert!(rendered.contains("SONG LIBRARY"));
        for song in StaffSong::ALL {
            assert!(rendered.contains(song.label()));
        }
        assert!(rendered.contains("Enter select"));
    }

    #[test]
    fn device_names_are_compacted_for_eighty_column_status() {
        let compact = compact_name("MacBook Pro Speakers", 16);
        assert_eq!(compact.chars().count(), 16);
        assert!(compact.ends_with('…'));
        assert_eq!(compact_name("MPKmini2", 12), "MPKmini2");
        assert!(FOOTER.chars().count() <= 80);
        assert!(STAFF_FOOTER.chars().count() <= 80);
    }

    #[test]
    fn help_overlay_exposes_naming_control_at_eighty_columns() {
        let now = Instant::now();
        let mut app = App::new(now, 0.7, 100, (48, 72));
        app.help = true;
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app, now)).unwrap();
        let rendered = format!("{:?}", terminal.backend().buffer());
        assert!(rendered.contains("Letters / Fixed Do"));
        assert!(rendered.contains("Rhythm:"));
    }
}
