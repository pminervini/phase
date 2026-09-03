use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "phase",
    version,
    about = "Offline terminal MIDI instrument and piano tutor"
)]
pub struct Cli {
    /// Select a MIDI input by case-insensitive substring
    #[arg(long, global = true)]
    pub midi_port: Option<String>,

    /// Select a CoreAudio output by case-insensitive substring
    #[arg(long, global = true)]
    pub audio_device: Option<String>,

    /// Run without opening an audio output
    #[arg(long, global = true)]
    pub no_audio: bool,

    /// Generate synthetic notes; no MIDI controller required
    #[arg(long, global = true)]
    pub demo: bool,

    /// Show nonfatal diagnostic details outside the TUI
    #[arg(long, global = true)]
    pub debug: bool,

    /// Run a noninteractive integration smoke test
    #[arg(long, hide = true, global = true)]
    pub smoke_test: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List MIDI and audio devices and default selections
    Devices,
    /// Print decoded MIDI input events
    Monitor {
        /// Stop after this many seconds; omit to run until Ctrl-C
        #[arg(long)]
        duration: Option<u64>,
    },
}
