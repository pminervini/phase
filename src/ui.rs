use crate::app::{App, Mode};
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
        .constraints(if compact {
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Min(5),
                Constraint::Length(1),
            ]
        } else {
            [
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(9),
                Constraint::Min(7),
                Constraint::Length(1),
            ]
        })
        .split(area);

    render_title(frame, app, chunks[0]);
    render_status(frame, app, now, chunks[1]);
    render_exercise(frame, app, chunks[2]);
    render_keyboard(frame, app, chunks[3]);
    render_lower(frame, app, chunks[4]);
    render_footer(frame, chunks[5]);
    if app.help {
        render_help(frame, area);
    }
}

fn cyber_block(title: &'static str) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            format!(" {title} "),
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
    let audio = if app.audio_ok { "online" } else { "offline" };
    let status = Line::from(vec![
        Span::styled("MIDI ", Style::default().fg(DIM)),
        Span::styled(&app.midi_name, Style::default().fg(CYAN)),
        Span::raw("  "),
        Span::styled("AUDIO ", Style::default().fg(DIM)),
        Span::styled(
            format!("{} ({audio})", app.audio_name),
            Style::default().fg(if app.audio_ok { CYAN } else { MAGENTA }),
        ),
        Span::raw("  "),
        Span::styled(format!("{} BPM", app.bpm), Style::default().fg(VIOLET)),
        Span::raw("  "),
        Span::styled(
            format!("SUS {}", if app.sustain { "ON" } else { "off" }),
            Style::default().fg(if app.sustain { MAGENTA } else { DIM }),
        ),
        Span::raw("  "),
        Span::styled(
            format!("VOL {:>3}%", (app.volume * 100.0).round()),
            Style::default().fg(CYAN),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:02}:{:02}", elapsed / 60, elapsed % 60),
            Style::default().fg(DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(status).block(cyber_block("SYSTEM")), area);
}

fn render_exercise(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (target, detail) = match app.mode {
        Mode::Freeplay => (
            app.chord
                .map_or_else(|| "—".into(), |chord| chord.to_string()),
            "Play freely · chord detection ignores inversions".into(),
        ),
        Mode::Notes => {
            let target = if app.note_exercise.hide_name {
                "●".into()
            } else {
                app.note_exercise.target.to_string()
            };
            let weak = app
                .note_exercise
                .weak_notes
                .iter()
                .max_by_key(|(_, count)| *count)
                .and_then(|(&value, &count)| {
                    crate::music::MidiNote::new(value)
                        .map(|note| format!(" · weak {note} ×{count}"))
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
        Mode::Scales => {
            let mistakes = app
                .scale_exercise
                .recent_mistakes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let completion = app.scale_exercise.last_completion.map_or_else(
                || "—".into(),
                |elapsed| format!("{:.1}s", elapsed.as_secs_f32()),
            );
            (
                app.scale_exercise.expected().to_string(),
                format!(
                    "{} {} · step {}/{} · {} complete · last {completion} · mistakes {}",
                    app.scale_exercise.root,
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
            app.rhythm_note.to_string(),
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

fn render_keyboard(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    let mut labels = Vec::new();
    for value in 48..=72 {
        let note = crate::music::MidiNote::new(value).expect("keyboard range is valid");
        let state = app.notes[usize::from(value)];
        let black = matches!(note.pitch_class().value(), 1 | 3 | 6 | 8 | 10);
        let active = state.velocity > 0;
        let brightness = 90_u8.saturating_add(state.velocity);
        let active_color = if black {
            Color::Rgb(brightness, 50, 210)
        } else {
            Color::Rgb(20, brightness, 230)
        };
        let key_style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(active_color)
                .add_modifier(Modifier::BOLD)
        } else if black {
            Style::default().fg(DIM).bg(Color::Rgb(24, 25, 38))
        } else {
            Style::default()
                .fg(Color::Rgb(170, 178, 193))
                .bg(Color::Rgb(40, 44, 55))
        };
        upper.push(Span::styled(if black { "███" } else { "   " }, key_style));
        lower.push(Span::styled(if black { " ║ " } else { " │ " }, key_style));
        labels.push(Span::styled(
            format!("{:^3}", note.pitch_class()),
            key_style,
        ));
    }
    let active: Vec<_> = app
        .active_notes()
        .map(|(note, state)| format!("{} [{:>3}] v{:>3}", note, note.value(), state.velocity))
        .collect();
    let lines = vec![
        Line::from(upper),
        Line::from(lower),
        Line::from(labels),
        Line::from(Span::styled(
            if active.is_empty() {
                "active: —".into()
            } else {
                format!("active: {}", active.join("  "))
            },
            Style::default().fg(CYAN),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(cyber_block("KEYBOARD · C3—C5"))
            .wrap(Wrap { trim: true }),
        area,
    );
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
        .map(|event| Line::from(Span::styled(event, Style::default().fg(DIM))))
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
                        .map_or_else(|| "—".into(), |chord| chord.to_string())
                ),
                Style::default().fg(MAGENTA),
            )),
            Line::from(Span::styled(
                "velocity-sensitive 32-voice electric piano",
                Style::default().fg(DIM),
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(text).block(cyber_block("SESSION")),
        columns[1],
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Paragraph::new(" q quit  tab mode  space pause  r restart  ←→ option  ↑↓ difficulty  +/- bpm  m mute  [/] volume  ? help ").style(Style::default().fg(DIM).bg(BG)).alignment(Alignment::Center), area);
}

fn render_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(68, 72, area);
    frame.render_widget(Clear, popup);
    let help = vec![
        Line::from(Span::styled(
            "PHASE CONTROL MATRIX",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("q quit      tab / shift-tab change mode      space pause/resume"),
        Line::from("r restart   ←/→ root or option   ↑/↓ scale or difficulty"),
        Line::from("+/- BPM     m mute              [/] master volume"),
        Line::from("h hide/show target note name (notes mode)    ? or esc close"),
        Line::from(""),
        Line::from(Span::styled(
            "MIDI keyboard input is independent of terminal keys.",
            Style::default().fg(CYAN),
        )),
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
