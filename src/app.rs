//! Application lifecycle: window creation, the wgpu surface, the audio stream
//! and the per-frame update/render loop, wired into winit's `ApplicationHandler`.

use std::sync::Arc;
use std::time::{Instant, SystemTime};

use anyhow::{Context, Result};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::audio::{spawn_audio, Analyzer, AudioConfig, AudioHandle, SAMPLE_RATE};
use crate::config::Config;
use crate::preset::{self, Preset};
use crate::render::{Renderer, Uniforms};

/// Everything tied to an active GPU surface. Created in `resumed`.
struct Graphics {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

impl Graphics {
    fn new(window: Arc<Window>, vsync: bool, prelude: String, first_body: &str) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .context("creating the render surface")?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .context("no suitable GPU adapter found (is Vulkan available?)")?;

        log::info!("GPU: {}", adapter.get_info().name);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kefirdrop-device"),
                ..Default::default()
            },
            None,
        ))
        .context("creating the GPU device")?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let present_mode = if vsync {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .iter()
                .copied()
                .find(|m| matches!(m, wgpu::PresentMode::Mailbox | wgpu::PresentMode::Immediate))
                .unwrap_or(wgpu::PresentMode::Fifo)
        };

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(
            &device,
            &queue,
            format,
            config.width,
            config.height,
            prelude,
            first_body,
        )?;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
        })
    }
}

pub struct App {
    config: Config,
    audio: AudioHandle,
    analyzer: Analyzer,
    presets: Vec<Preset>,
    preset_idx: usize,
    prelude: String,
    uniforms: Uniforms,
    gfx: Option<Graphics>,
    start: Instant,
    last_frame: Instant,
    /// Last seen modification time of the active shader, for hot-reload.
    shader_mtime: Option<SystemTime>,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        let presets = preset::load_all(&config.assets_dir)?;
        let preset_idx = config
            .start_preset
            .as_deref()
            .and_then(|name| presets.iter().position(|p| p.spec.name == name))
            .unwrap_or(0);

        let prelude = std::fs::read_to_string(config.assets_dir.join("shaders/prelude.wgsl"))
            .context("reading shaders/prelude.wgsl")?;

        let audio = spawn_audio(AudioConfig {
            synthetic: config.synthetic,
            device: config.device.clone(),
            sample_rate: SAMPLE_RATE,
        })?;

        let now = Instant::now();
        Ok(Self {
            config,
            audio,
            analyzer: Analyzer::new(),
            presets,
            preset_idx,
            prelude,
            uniforms: Uniforms::default(),
            gfx: None,
            start: now,
            last_frame: now,
            shader_mtime: None,
        })
    }

    /// Move to a different preset (wrapping) and apply it.
    fn switch_preset(&mut self, delta: i32) {
        let n = self.presets.len() as i32;
        self.preset_idx = (((self.preset_idx as i32 + delta) % n + n) % n) as usize;
        self.apply_current_preset();
    }

    /// Compile and bind the current preset's shader, refreshing the mtime.
    fn apply_current_preset(&mut self) {
        let preset = &self.presets[self.preset_idx];
        match preset.load_body() {
            Ok(body) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    match gfx.renderer.set_preset(&gfx.device, &body) {
                        Ok(()) => {
                            log::info!("preset: {} — {}", preset.spec.name, preset.spec.description)
                        }
                        Err(e) => {
                            log::error!("preset `{}` failed to compile: {e:#}", preset.spec.name)
                        }
                    }
                }
                self.shader_mtime = std::fs::metadata(&preset.shader_path)
                    .and_then(|m| m.modified())
                    .ok();
            }
            Err(e) => log::error!("{e:#}"),
        }
    }

    /// Reload the active shader if its file changed on disk.
    fn check_hot_reload(&mut self) {
        let path = self.presets[self.preset_idx].shader_path.clone();
        let changed = match (
            std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
            self.shader_mtime,
        ) {
            (Some(modified), Some(prev)) => modified > prev,
            _ => false,
        };
        if changed {
            log::info!("reloading {}", path.display());
            self.apply_current_preset();
        }
    }

    /// Update analysis and render one frame.
    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().clamp(0.0, 0.1);
        self.last_frame = now;
        let time = (now - self.start).as_secs_f32();

        let samples = self.audio.samples();
        let features = self.analyzer.process(samples, dt);

        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        let resolution = [gfx.config.width as f32, gfx.config.height as f32];
        self.uniforms.update(resolution, time, features);

        match gfx.surface.get_current_texture() {
            Ok(frame) => {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                gfx.renderer
                    .render(&gfx.device, &gfx.queue, &view, &self.uniforms);
                gfx.window.pre_present_notify();
                frame.present();
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gfx.surface.configure(&gfx.device, &gfx.config);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("GPU out of memory; exiting");
            }
            Err(e) => log::warn!("surface error: {e}"),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("kefirdrop")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let first_body = match self.presets[self.preset_idx].load_body() {
            Ok(b) => b,
            Err(e) => {
                log::error!("{e:#}");
                event_loop.exit();
                return;
            }
        };

        match Graphics::new(window, self.config.vsync, self.prelude.clone(), &first_body) {
            Ok(gfx) => {
                let p = &self.presets[self.preset_idx].spec;
                log::info!("preset: {} — {}", p.name, p.description);
                self.gfx = Some(gfx);
                self.shader_mtime = std::fs::metadata(&self.presets[self.preset_idx].shader_path)
                    .and_then(|m| m.modified())
                    .ok();
            }
            Err(e) => {
                log::error!("graphics init failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.config.width = size.width.max(1);
                    gfx.config.height = size.height.max(1);
                    gfx.surface.configure(&gfx.device, &gfx.config);
                    gfx.renderer.resize(
                        &gfx.device,
                        &gfx.queue,
                        gfx.config.width,
                        gfx.config.height,
                    );
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => event_loop.exit(),
                Key::Named(NamedKey::ArrowRight) => self.switch_preset(1),
                Key::Named(NamedKey::ArrowLeft) => self.switch_preset(-1),
                Key::Character(c) => match c.as_str() {
                    "q" | "Q" => event_loop.exit(),
                    "n" | "N" => self.switch_preset(1),
                    "p" | "P" => self.switch_preset(-1),
                    "r" | "R" => self.apply_current_preset(),
                    _ => {}
                },
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                self.check_hot_reload();
                self.frame();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gfx) = &self.gfx {
            gfx.window.request_redraw();
        }
    }
}
