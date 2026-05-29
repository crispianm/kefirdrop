//! Runtime configuration, parsed from CLI arguments.

use std::path::PathBuf;

/// How kefirdrop should behave for this run.
#[derive(Clone, Debug)]
pub struct Config {
    /// Use the built-in synthetic audio source instead of capturing real audio.
    /// Handy for development on machines without a sound server.
    pub synthetic: bool,
    /// Explicit capture device (PipeWire/PulseAudio source name). When `None`,
    /// the default sink's `.monitor` is auto-detected via `pactl`. Can also be
    /// set with the `KEFIRDROP_AUDIO_DEVICE` environment variable.
    pub device: Option<String>,
    /// Cap the frame rate to the display refresh (FIFO present mode).
    pub vsync: bool,
    /// Directory containing `shaders/` and `presets/`.
    pub assets_dir: PathBuf,
    /// Name of the preset to start on (matches a preset's `name`). When `None`,
    /// the first preset alphabetically is used.
    pub start_preset: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            synthetic: false,
            device: None,
            vsync: true,
            assets_dir: default_assets_dir(),
            start_preset: None,
        }
    }
}

impl Config {
    /// Parse configuration from `std::env::args`. Returns `None` if the program
    /// should exit immediately (e.g. after printing `--help`).
    pub fn from_args() -> Option<Self> {
        let mut cfg = Config::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--synthetic" => cfg.synthetic = true,
                "--no-vsync" => cfg.vsync = false,
                "--device" => cfg.device = args.next(),
                "--preset" => cfg.start_preset = args.next(),
                "--assets" => {
                    if let Some(dir) = args.next() {
                        cfg.assets_dir = PathBuf::from(dir);
                    }
                }
                "-h" | "--help" => {
                    print_help();
                    return None;
                }
                other => {
                    eprintln!("kefirdrop: unknown argument `{other}` (try --help)");
                    return None;
                }
            }
        }
        Some(cfg)
    }
}

/// Locate the bundled `assets/` directory: prefer one next to the executable,
/// otherwise fall back to `assets/` relative to the working directory (cargo run).
fn default_assets_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("assets");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from("assets")
}

fn print_help() {
    println!(
        "kefirdrop — a GPU music visualizer for Linux audiophiles\n\
         \n\
         USAGE:\n    kefirdrop [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20   --synthetic        Use a built-in test signal instead of real audio\n\
         \x20   --device <NAME>    Capture from a specific PipeWire/Pulse source\n\
         \x20   --no-vsync         Disable vsync (uncapped frame rate)\n\
         \x20   --preset <NAME>    Start on a named preset\n\
         \x20   --assets <DIR>     Path to the assets directory (shaders + presets)\n\
         \x20   -h, --help         Show this help\n\
         \n\
         KEYS (while running):\n\
         \x20   Right / N          Next preset\n\
         \x20   Left  / P          Previous preset\n\
         \x20   R                  Reload current shader from disk\n\
         \x20   Esc / Q            Quit\n\
         \n\
         ENV:\n\
         \x20   KEFIRDROP_AUDIO_DEVICE   Override the capture source name"
    );
}
