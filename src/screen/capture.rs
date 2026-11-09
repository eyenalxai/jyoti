use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use gl::types::{GLint, GLsizei, GLuint};
use libwayshot::{WayshotConnection, WayshotTarget};
use r_egl_wayland::{EGL_INSTALCE, WayEglTrait, r_egl};
use rune::Waker;
use wayland_client::Connection;

use crate::screen::grid::prepare_frame;
use crate::screen::monitor::{MonitorInfo, Scene};
use crate::screen::reducer::{ReducerState, reduce_prepared};
use crate::screen::sync::{SyncConfig, SyncFrame, SyncRun};

const TICK: Duration = Duration::from_millis(667);
const GRID_WIDTH: usize = 64;
const GRID_HEIGHT: usize = 36;

#[derive(Clone, Debug)]
pub enum ScreenCommand {
    ListMonitors,
    Start { monitor: String, config: SyncConfig },
    Configure(SyncConfig),
    Pause,
    Resume,
    SelectMonitor(String),
    Stop,
    CaptureOnce,
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum ScreenEvent {
    Monitors(Vec<MonitorInfo>),
    Scene(Scene),
    Started,
    Stopped { drove: Vec<String> },
    Error(String),
    Fatal(String),
}

pub struct ScreenHandle {
    pub commands: Sender<ScreenCommand>,
    pub events: Receiver<ScreenEvent>,
    pub latest: Arc<Mutex<Option<SyncFrame>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for ScreenHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(ScreenCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl ScreenHandle {
    pub fn start(waker: Waker) -> Self {
        let (commands, inbox) = mpsc::channel::<ScreenCommand>();
        let (outbox, events) = mpsc::channel::<ScreenEvent>();
        let latest = Arc::new(Mutex::new(None));
        let thread_latest = Arc::clone(&latest);
        let join = thread::Builder::new()
            .name("jyoti-screen".to_owned())
            .spawn(move || {
                run(inbox, outbox, waker, thread_latest);
            })
            .ok();
        Self {
            commands,
            events,
            latest,
            join,
        }
    }

    pub fn send(&self, command: ScreenCommand) -> bool {
        self.commands.send(command).is_ok()
    }
}

fn run(
    inbox: Receiver<ScreenCommand>,
    events: Sender<ScreenEvent>,
    waker: Waker,
    latest: Arc<Mutex<Option<SyncFrame>>>,
) {
    let mut backend = match Backend::connect() {
        Ok(backend) => backend,
        Err(error) => {
            let _ = events.send(ScreenEvent::Fatal(error));
            waker();
            return;
        }
    };

    let _ = events.send(ScreenEvent::Monitors(backend.monitors()));
    waker();

    let mut running = false;
    let mut paused = false;
    let mut run: Option<SyncRun> = None;
    let mut selected = backend.first_monitor();
    let mut reducer = ReducerState::default();
    let mut last_capture = Instant::now() - TICK;

    loop {
        let wait = if running && !paused {
            TICK.saturating_sub(last_capture.elapsed())
        } else {
            Duration::from_secs(3_600)
        };

        let command = match inbox.recv_timeout(wait) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        match command {
            Some(ScreenCommand::ListMonitors) => {
                let _ = events.send(ScreenEvent::Monitors(backend.monitors()));
                waker();
            }
            Some(ScreenCommand::Start { monitor, config }) => {
                selected = Some(monitor);
                run = Some(SyncRun::new(config));
                if let Ok(mut slot) = latest.lock() {
                    *slot = None;
                }
                running = true;
                paused = false;
                let _ = events.send(ScreenEvent::Started);
                waker();
                capture(
                    &mut backend,
                    selected.as_deref(),
                    &events,
                    &waker,
                    &mut reducer,
                    run.as_mut(),
                    &latest,
                );
                last_capture = Instant::now();
            }
            Some(ScreenCommand::Configure(config)) => {
                if let Some(run) = run.as_mut() {
                    run.configure(config);
                }
            }
            Some(ScreenCommand::Pause) => {
                paused = true;
            }
            Some(ScreenCommand::Resume) => {
                paused = false;
                last_capture = Instant::now() - TICK;
            }
            Some(ScreenCommand::SelectMonitor(name)) => {
                selected = Some(name);
                last_capture = Instant::now() - TICK;
            }
            Some(ScreenCommand::Stop) => {
                running = false;
                paused = false;
                let drove = finish_run(&mut run);
                let _ = events.send(ScreenEvent::Stopped { drove });
                waker();
            }
            Some(ScreenCommand::CaptureOnce) => {
                capture(
                    &mut backend,
                    selected.as_deref(),
                    &events,
                    &waker,
                    &mut reducer,
                    None,
                    &latest,
                );
            }
            Some(ScreenCommand::Shutdown) => {
                let drove = finish_run(&mut run);
                let _ = events.send(ScreenEvent::Stopped { drove });
                break;
            }
            None => {
                if !running || paused {
                    continue;
                }
                capture(
                    &mut backend,
                    selected.as_deref(),
                    &events,
                    &waker,
                    &mut reducer,
                    run.as_mut(),
                    &latest,
                );
                last_capture = Instant::now();
            }
        }
    }
}

fn finish_run(run: &mut Option<SyncRun>) -> Vec<String> {
    run.take()
        .map(|run| run.drove_brightness())
        .unwrap_or_default()
}

fn capture(
    capture: &mut dyn Capture,
    selected: Option<&str>,
    events: &Sender<ScreenEvent>,
    waker: &Waker,
    reducer: &mut ReducerState,
    run: Option<&mut SyncRun>,
    latest: &Arc<Mutex<Option<SyncFrame>>>,
) {
    let Some(name) = selected else {
        let _ = events.send(ScreenEvent::Error("No monitor is selected.".to_owned()));
        waker();
        return;
    };

    match capture.capture(name, reducer) {
        Ok(scene) => {
            match run {
                Some(run) => {
                    let frame = run.apply(scene, Instant::now());
                    if let Ok(mut slot) = latest.lock() {
                        *slot = Some(frame);
                    }
                }
                None => {
                    let _ = events.send(ScreenEvent::Scene(scene));
                }
            }
            waker();
        }
        Err(error) => {
            let _ = events.send(ScreenEvent::Error(error));
            waker();
        }
    }
}

pub(crate) trait Capture {
    fn monitors(&mut self) -> Vec<MonitorInfo>;
    fn first_monitor(&self) -> Option<String>;
    fn capture(&mut self, monitor: &str, reducer: &mut ReducerState) -> Result<Scene, String>;
}

struct Backend {
    wayshot: WayshotConnection,
    downsampler: Downsampler,
}

impl Backend {
    fn connect() -> Result<Self, String> {
        let connection = Connection::connect_to_env()
            .map_err(|error| format!("Could not connect to Wayland: {error}"))?;
        let render_node = render_node()
            .ok_or_else(|| "No DRM render node was found under /dev/dri.".to_owned())?;
        let wayshot = WayshotConnection::from_connection_with_dmabuf(connection, render_node)
            .map_err(|error| format!("Could not start Wayland capture: {error}"))?;
        let display = EGL_INSTALCE
            .get_display_wl(&wayshot.conn.display())
            .map_err(|error| format!("Could not get the EGL display: {error}"))?;
        let downsampler = Downsampler::new(display)?;
        Ok(Self {
            wayshot,
            downsampler,
        })
    }

    fn target(&self, name: &str) -> Option<(WayshotTarget, usize, usize)> {
        let output = self
            .wayshot
            .get_all_outputs()
            .iter()
            .find(|output| output.name == name)?;
        Some((
            WayshotTarget::Screen(output.wl_output.clone()),
            output.physical_size.width as usize,
            output.physical_size.height as usize,
        ))
    }

    fn capture_frame(
        &mut self,
        target: &WayshotTarget,
        width: usize,
        height: usize,
        reducer: &mut ReducerState,
    ) -> Result<Scene, String> {
        let pixels = self
            .downsampler
            .capture(&self.wayshot, target, width, height)?;
        let prepared = prepare_frame(GRID_WIDTH, GRID_HEIGHT, &pixels);
        let reduction = reduce_prepared(&prepared, reducer);
        *reducer = reduction.state;
        Ok(Scene {
            color: reduction.derived.rgb,
            brightness: reduction.derived.scene_brightness,
        })
    }
}

impl Capture for Backend {
    fn monitors(&mut self) -> Vec<MonitorInfo> {
        let _ = self.wayshot.refresh_outputs();
        self.wayshot
            .get_all_outputs()
            .iter()
            .map(|output| MonitorInfo {
                name: output.name.clone(),
                description: output.description.clone(),
                width: output.physical_size.width,
                height: output.physical_size.height,
            })
            .collect()
    }

    fn first_monitor(&self) -> Option<String> {
        self.wayshot
            .get_all_outputs()
            .first()
            .map(|output| output.name.clone())
    }

    fn capture(&mut self, monitor: &str, reducer: &mut ReducerState) -> Result<Scene, String> {
        let Some((target, width, height)) = self.target(monitor) else {
            return Err(format!("The monitor {monitor} is no longer available."));
        };
        self.capture_frame(&target, width, height, reducer)
    }
}

fn render_node() -> Option<PathBuf> {
    std::fs::read_dir("/dev/dri")
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("renderD"))
        })
}

struct Downsampler {
    _display: r_egl::Display,
    _context: r_egl::Context,
    _surface: r_egl::Surface,
    source_texture: GLuint,
    destination_texture: GLuint,
    source_framebuffer: GLuint,
    destination_framebuffer: GLuint,
}

impl Downsampler {
    fn new(display: r_egl::Display) -> Result<Self, String> {
        EGL_INSTALCE
            .bind_api(r_egl::OPENGL_API)
            .map_err(|error| format!("Could not select the OpenGL API: {error}"))?;
        EGL_INSTALCE
            .initialize(display)
            .map_err(|error| format!("Could not initialize EGL: {error}"))?;

        let config = EGL_INSTALCE
            .choose_first_config(
                display,
                &[
                    r_egl::RED_SIZE,
                    8,
                    r_egl::GREEN_SIZE,
                    8,
                    r_egl::BLUE_SIZE,
                    8,
                    r_egl::ALPHA_SIZE,
                    8,
                    r_egl::NONE,
                ],
            )
            .map_err(|error| format!("Could not choose an EGL config: {error}"))?
            .ok_or_else(|| "No EGL config was available.".to_owned())?;
        let context = EGL_INSTALCE
            .create_context(
                display,
                config,
                None,
                &[
                    r_egl::CONTEXT_MAJOR_VERSION,
                    3,
                    r_egl::CONTEXT_MINOR_VERSION,
                    3,
                    r_egl::CONTEXT_OPENGL_PROFILE_MASK,
                    r_egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    r_egl::NONE,
                ],
            )
            .map_err(|error| format!("Could not create an EGL context: {error}"))?;
        let surface = EGL_INSTALCE
            .create_pbuffer_surface(
                display,
                config,
                &[r_egl::WIDTH, 1, r_egl::HEIGHT, 1, r_egl::NONE],
            )
            .map_err(|error| format!("Could not create an EGL surface: {error}"))?;
        EGL_INSTALCE
            .make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|error| format!("Could not bind the EGL context: {error}"))?;

        gl::load_with(|symbol| {
            EGL_INSTALCE
                .get_proc_address(symbol)
                .map_or(std::ptr::null(), |function| function as *const c_void)
        });

        let mut downsampler = Self {
            _display: display,
            _context: context,
            _surface: surface,
            source_texture: 0,
            destination_texture: 0,
            source_framebuffer: 0,
            destination_framebuffer: 0,
        };
        downsampler.prepare();
        Ok(downsampler)
    }

    fn prepare(&mut self) {
        unsafe {
            gl::GenTextures(1, &mut self.source_texture);
            gl::BindTexture(gl::TEXTURE_2D, self.source_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as GLint,
            );

            gl::GenTextures(1, &mut self.destination_texture);
            gl::BindTexture(gl::TEXTURE_2D, self.destination_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as GLint,
                GRID_WIDTH as GLsizei,
                GRID_HEIGHT as GLsizei,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );

            gl::GenFramebuffers(1, &mut self.source_framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.source_framebuffer);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.source_texture,
                0,
            );
            gl::ReadBuffer(gl::COLOR_ATTACHMENT0);

            gl::GenFramebuffers(1, &mut self.destination_framebuffer);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.destination_framebuffer);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                self.destination_texture,
                0,
            );
            gl::DrawBuffer(gl::COLOR_ATTACHMENT0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
    }

    fn capture(
        &mut self,
        wayshot: &WayshotConnection,
        target: &WayshotTarget,
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>, String> {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.source_texture);
        }
        wayshot
            .bind_target_frame_to_gl_texture(target, false, None)
            .map_err(|error| format!("Could not capture the monitor: {error}"))?;

        let mut pixels = vec![0u8; GRID_WIDTH * GRID_HEIGHT * 4];
        unsafe {
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, self.source_framebuffer);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, self.destination_framebuffer);
            gl::BlitFramebuffer(
                0,
                0,
                width as GLint,
                height as GLint,
                0,
                0,
                GRID_WIDTH as GLint,
                GRID_HEIGHT as GLint,
                gl::COLOR_BUFFER_BIT,
                gl::LINEAR,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.destination_framebuffer);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadPixels(
                0,
                0,
                GRID_WIDTH as GLsizei,
                GRID_HEIGHT as GLsizei,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut c_void,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            let error = gl::GetError();
            if error != gl::NO_ERROR {
                return Err(format!("The GPU downsample failed ({error:#X})."));
            }
        }
        Ok(pixels)
    }
}

impl Drop for Downsampler {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.source_framebuffer);
            gl::DeleteFramebuffers(1, &self.destination_framebuffer);
            gl::DeleteTextures(1, &self.source_texture);
            gl::DeleteTextures(1, &self.destination_texture);
        }
        let _ = EGL_INSTALCE.make_current(self._display, None, None, None);
        let _ = EGL_INSTALCE.destroy_surface(self._display, self._surface);
        let _ = EGL_INSTALCE.destroy_context(self._display, self._context);
    }
}
