//! kefirdrop — a GPU-accelerated, real-time music visualizer for Linux
//! audiophiles, inspired by MilkDrop.

mod app;
mod audio;
mod config;
mod preset;
mod render;

use anyhow::Result;

use app::App;
use config::Config;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let Some(config) = Config::from_args() else {
        return Ok(());
    };

    let event_loop = winit::event_loop::EventLoop::new()?;
    // Poll continuously so the visualizer animates even without input events.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App::new(config)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
