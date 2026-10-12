mod jobs;
mod preferences;
use eframe::egui::{self, ViewportCommand};
use jobs::{Jobs, Request, Response};
use kedr_ui::{Effect, Shell, Theme, apply_theme};
use kedr_workbench::TaskRunner;
use preferences::Preferences;
use std::{path::PathBuf, sync::atomic::Ordering};
#[derive(Default)]
struct Options {
    project: Option<PathBuf>,
    capture: Option<PathBuf>,
    state: String,
    light: bool,
    size: Option<[f32; 2]>,
}
impl Options {
    fn parse() -> Result<Self, String> {
        let mut v = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--capture" => v.capture = Some(args.next().ok_or("Missing capture path")?.into()),
                "--state" => v.state = args.next().ok_or("Missing state")?,
                "--light" => v.light = true,
                "--size" => {
                    let size = args.next().ok_or("Missing size")?;
                    let (a, b) = size.split_once('x').ok_or("Use WIDTHxHEIGHT")?;
                    let (w, h): (f32, f32) = (
                        a.parse().map_err(|_| "Invalid width")?,
                        b.parse().map_err(|_| "Invalid height")?,
                    );
                    if !w.is_finite()
                        || !h.is_finite()
                        || !(760.0..=4096.0).contains(&w)
                        || !(520.0..=4096.0).contains(&h)
                    {
                        return Err("Size must be 760–4096 by 520–4096".into());
                    }
                    v.size = Some([w, h]);
                }
                "--help" => {
                    println!(
                        "Kedr Studio [workspace]\n--capture FILE --state editor|welcome|palette|commands|settings|search|unsaved|trust|error\n--light --size WIDTHxHEIGHT\nCapture mode never saves preferences or runs tasks."
                    );
                    std::process::exit(0)
                }
                s if s.starts_with('-') => return Err(format!("Unknown option: {s}")),
                _ => {
                    if v.project.is_some() {
                        return Err("Supply one workspace path".into());
                    }
                    v.project = Some(arg.into());
                }
            }
        }
        Ok(v)
    }
}
struct Studio {
    shell: Shell,
    jobs: Jobs,
    options: Options,
    frames: usize,
    state_applied: bool,
    capture_requested: bool,
    quit_allowed: bool,
}
impl Studio {
    fn new(cc: &eframe::CreationContext<'_>, options: Options) -> Self {
        let prefs = if options.capture.is_none() {
            Preferences::read()
        } else {
            Preferences::default()
        };
        let mut settings = prefs.settings();
        if options.light {
            settings.theme = Theme::Light;
        }
        apply_theme(&cc.egui_ctx, settings.theme);
        let mut shell = Shell::new("Kedr Studio", settings);
        shell.recent = prefs
            .recent
            .into_iter()
            .filter(|p| p.is_dir())
            .take(8)
            .collect();
        let jobs = Jobs::new(cc.egui_ctx.clone());
        let project = options.project.clone();
        let mut app = Self {
            shell,
            jobs,
            options,
            frames: 0,
            state_applied: false,
            capture_requested: false,
            quit_allowed: false,
        };
        if let Some(path) = project {
            app.open_project(path);
        }
        app
    }
    fn open_project(&mut self, path: PathBuf) {
        let id = self.jobs.generation.fetch_add(1, Ordering::Relaxed) + 1;
        self.jobs.search_generation.fetch_add(1, Ordering::Relaxed);
        self.shell.loading = true;
        self.shell.search_loading = false;
        self.send(Request::Scan(path, id));
    }
    fn send(&mut self, r: Request) {
        if let Err(e) = self.jobs.send.try_send(r) {
            self.shell.loading = false;
            self.shell.search_loading = false;
            self.shell
                .error(format!("Background queue unavailable: {e}"));
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.jobs.receive.try_recv() {
            let current = self.jobs.generation.load(Ordering::Relaxed);
            match r {
                Response::Scanned(id, result) if id == current => match result {
                    Ok(w) => {
                        let root = w.project.root.clone();
                        let initial = [
                            root.join("src/main.rs"),
                            root.join("src/lib.rs"),
                            root.join("README.md"),
                        ]
                        .into_iter()
                        .find(|p| w.project.files.contains(p))
                        .or_else(|| w.project.files.first().cloned());
                        self.shell.set_workspace(w);
                        self.shell.recent.retain(|p| p != &root);
                        self.shell.recent.insert(0, root);
                        self.shell.recent.truncate(8);
                        ctx.send_viewport_cmd(ViewportCommand::Title(format!(
                            "{} — Kedr Studio",
                            self.shell.workspace.as_ref().unwrap().name()
                        )));
                        if let Some(path) = initial {
                            self.send(Request::Open(path, current));
                        }
                    }
                    Err(e) => {
                        self.shell.loading = false;
                        self.shell.error(e);
                    }
                },
                Response::Opened(id, result) if id == current => match result {
                    Ok(doc) => self.shell.accept_document(doc),
                    Err(e) => self.shell.error(e),
                },
                Response::Searched(id, search_id, r)
                    if id == current
                        && search_id == self.jobs.search_generation.load(Ordering::Relaxed) =>
                {
                    self.shell.search_loading = false;
                    self.shell.search_report = Some(r);
                }
                _ => {}
            }
        }
    }
    fn preferences(&self) -> Preferences {
        Preferences {
            theme: if self.shell.settings.theme == Theme::Light {
                "light"
            } else {
                "dark"
            }
            .into(),
            font_size: self.shell.settings.font_size,
            recent: self.shell.recent.clone(),
        }
    }
    fn effects(&mut self, effects: Vec<Effect>, ctx: &egui::Context) {
        for e in effects {
            match e {
                Effect::PickProject => {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Kedr workspace")
                        .pick_folder()
                    {
                        self.shell.request_project(path);
                        ctx.request_repaint();
                    }
                }
                Effect::OpenProject(path) => self.open_project(path),
                Effect::OpenFile(path) => {
                    if !self.shell.activate_path(&path) {
                        self.send(Request::Open(
                            path,
                            self.jobs.generation.load(Ordering::Relaxed),
                        ));
                    }
                }
                Effect::Search(query) => {
                    if let Some(w) = &self.shell.workspace {
                        let id = self.jobs.generation.load(Ordering::Relaxed);
                        let search_id =
                            self.jobs.search_generation.fetch_add(1, Ordering::Relaxed) + 1;
                        self.send(Request::Search(
                            w.project.root.clone(),
                            w.project.files.clone(),
                            query,
                            id,
                            search_id,
                        ));
                    }
                }
                Effect::RunTask(kind) => {
                    if self.options.capture.is_some() {
                        self.shell
                            .error("Task execution is disabled in capture mode");
                        continue;
                    }
                    if let Some(w) = &self.shell.workspace {
                        let ctx = ctx.clone();
                        match TaskRunner::start(&w.project.root, kind, move || {
                            ctx.request_repaint()
                        }) {
                            Ok(t) => {
                                self.shell.task = Some(t);
                                self.shell.settings.show_output = true;
                            }
                            Err(e) => self.shell.error(e),
                        }
                    }
                }
                Effect::Quit => {
                    self.quit_allowed = true;
                    if self.options.capture.is_none()
                        && let Err(e) = self.preferences().write()
                    {
                        self.shell.error(format!("Preferences were not saved: {e}"));
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Effect::PreferencesChanged => {
                    if self.options.capture.is_none()
                        && let Err(e) = self.preferences().write()
                    {
                        self.shell.error(format!("Preferences were not saved: {e}"));
                    }
                }
            }
        }
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.frames += 1;
        self.poll(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit_allowed {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.shell.request_quit();
        }
        let ready = self.options.project.is_none()
            || self
                .shell
                .workspace
                .as_ref()
                .is_some_and(|w| !w.tabs.is_empty());
        if self.options.capture.is_some() && ready && !self.state_applied {
            self.shell.capture_state(&self.options.state);
            self.state_applied = true;
        }
        let effects = self.shell.show(ctx);
        self.effects(effects, ctx);
        if let Some(path) = &self.options.capture {
            if ready
                && self.state_applied
                && !self.shell.search_loading
                && self.frames > 8
                && !self.capture_requested
            {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
                self.capture_requested = true;
            }
            let image = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = image {
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let buffer =
                    image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, pixels)
                        .unwrap();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match buffer.save(path) {
                    Ok(()) => println!(
                        "Captured {} ({}x{}), {} visible editor rows",
                        path.display(),
                        image.size[0],
                        image.size[1],
                        self.shell.editor_cache.painted_rows
                    ),
                    Err(e) => eprintln!("Capture failed: {e}"),
                }
                self.quit_allowed = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if self.frames > 220 {
                eprintln!("Capture deadline exceeded");
                self.quit_allowed = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }
}
fn main() -> eframe::Result {
    let options = Options::parse().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(options.size.unwrap_or([1440., 900.]))
        .with_min_inner_size([760., 520.])
        .with_title("Kedr Studio");
    eframe::run_native(
        "Kedr Studio",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(Studio::new(cc, options)))),
    )
}
