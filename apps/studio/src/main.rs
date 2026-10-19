mod icon;
mod jobs;
mod picker;
mod preferences;
mod recovery;
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
    no_recovery: bool,
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
                "--no-recovery" => v.no_recovery = true,
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
                        "CodeCedar — Kedr [workspace]\n--capture FILE --state editor|welcome|palette|commands|settings|search|unsaved|trust|error\n--light --no-recovery --size WIDTHxHEIGHT\nCapture mode never saves preferences or runs tasks."
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
    picker: picker::FolderPicker,
    options: Options,
    frames: usize,
    state_applied: bool,
    capture_requested: bool,
    capture_started: Option<std::time::Instant>,
    quit_allowed: bool,
    disk_poll_at: std::time::Instant,
    disk_poll_pending: bool,
    recovery: Option<recovery::Recovery>,
    recovery_at: std::time::Instant,
    recovery_checkpoint: Option<kedr_workbench::recovery::Guard>,
    recovery_restore: Option<kedr_workbench::recovery::Guard>,
    finishing: bool,
    finish_sent: bool,
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
        if options.capture.is_some() {
            // Automated layout captures must not depend on the host pointer position.
            // Normal interactive sessions keep the standard tooltip behavior.
            cc.egui_ctx
                .style_mut(|style| style.interaction.tooltip_delay = 3600.);
        }
        let mut shell = Shell::new("CodeCedar — Kedr", settings);
        shell.recent = prefs
            .recent
            .into_iter()
            .filter(|p| p.is_dir())
            .take(8)
            .collect();
        let jobs = Jobs::new(cc.egui_ctx.clone());
        let project = options.project.clone();
        let recovery = if options.capture.is_none() && !options.no_recovery {
            match recovery::Recovery::start(cc.egui_ctx.clone()) {
                Ok(r) => Some(r),
                Err(e) => {
                    shell.error(e);
                    None
                }
            }
        } else {
            None
        };
        let mut app = Self {
            shell,
            jobs,
            picker: Default::default(),
            options,
            frames: 0,
            state_applied: false,
            capture_requested: false,
            capture_started: None,
            quit_allowed: false,
            disk_poll_at: std::time::Instant::now(),
            disk_poll_pending: false,
            recovery,
            recovery_at: std::time::Instant::now(),
            recovery_checkpoint: None,
            recovery_restore: None,
            finishing: false,
            finish_sent: false,
        };
        if let Some(path) = project {
            app.open_project(path);
        }
        app
    }
    fn open_project(&mut self, path: PathBuf) {
        self.shell.cancel_file_reload();
        self.shell.cancel_definition();
        self.recovery_restore = None;
        self.disk_poll_pending = false;
        self.disk_poll_at = std::time::Instant::now();
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
        while let Some(r) = self
            .recovery
            .as_ref()
            .and_then(|r| r.receive.try_recv().ok())
        {
            match r {
                recovery::Response::Offers(offers) => self.shell.recovery_offers = offers,
                recovery::Response::Error(e) => {
                    self.recovery_checkpoint = None;
                    self.shell.error(e)
                }
                recovery::Response::Restored(result) => {
                    self.shell.recovery_busy = false;
                    if self.finishing {
                        self.recovery_restore = None;
                        continue;
                    }
                    let guard = self.recovery_restore.take();
                    let result = result.and_then(|tabs| {
                        guard
                            .as_ref()
                            .ok_or_else(|| "Recovery request expired".into())
                            .and_then(|g| self.shell.accept_recovery(g, tabs))
                    });
                    if let Err(e) = result {
                        self.shell.error(e)
                    }
                }
                recovery::Response::Forgotten(id, result) => {
                    self.shell.recovery_busy = false;
                    match result {
                        Ok(()) => self.shell.recovery_offers.retain(|o| o.id != id),
                        Err(e) => self.shell.error(e),
                    }
                }
                recovery::Response::Finished(result) => match result {
                    Ok(()) => {
                        self.quit_allowed = true;
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                    Err(e) => {
                        self.finishing = false;
                        self.finish_sent = false;
                        self.shell
                            .error(format!("Recovery shutdown failed; window kept open: {e}"));
                    }
                },
            }
        }

        if let Some(result) = self.picker.poll() {
            match result {
                Ok((generation, Some(path)))
                    if generation == self.jobs.generation.load(Ordering::Relaxed) =>
                {
                    self.shell.request_project(path)
                }
                Ok(_) => {}
                Err(error) => self.shell.error(error),
            }
        }
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
                            "{} — CodeCedar — Kedr",
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
                Response::Probed(id, query, state) if id == current => {
                    self.disk_poll_pending = false;
                    self.shell.apply_disk_probe(&query, state);
                }
                Response::Reloaded(id, query, result) if id == current => {
                    self.shell.complete_reload(&query, result);
                }
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
            keymap: self.shell.settings.keymap.id().into(),
            shortcuts: self.shell.settings.shortcuts.clone(),
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
                    if !self
                        .picker
                        .start(self.jobs.generation.load(Ordering::Relaxed), ctx.clone())
                    {
                        self.shell.error(
                            "A folder picker is already open. Finish or cancel that dialog first.",
                        );
                    }
                }
                Effect::OpenProject(path) => self.open_project(path),
                Effect::ReloadFile(query) => {
                    if self.options.capture.is_some() {
                        self.shell.complete_reload(
                            &query,
                            Err("Disk reload is disabled in capture mode".into()),
                        );
                        continue;
                    }
                    let retry = query.clone();
                    if let Err(error) = self.jobs.send.try_send(Request::Reload(
                        query,
                        self.jobs.generation.load(Ordering::Relaxed),
                    )) {
                        self.shell.complete_reload(
                            &retry,
                            Err(format!("Reload queue unavailable: {error}")),
                        );
                    }
                }
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
                Effect::References => {
                    if self.options.capture.is_some() {
                        self.shell
                            .error("Language server execution is disabled in capture mode");
                        continue;
                    }
                    match self.shell.definition_query() {
                        Ok(query) => {
                            let ctx = ctx.clone();
                            self.shell
                                .set_references(kedr_workbench::lsp::ReferenceRunner::start(
                                    query,
                                    move || ctx.request_repaint(),
                                ));
                        }
                        Err(error) => self.shell.error(error),
                    }
                }
                Effect::Definition => {
                    if self.options.capture.is_some() {
                        self.shell
                            .error("Language server execution is disabled in capture mode");
                        continue;
                    }
                    match self.shell.definition_query() {
                        Ok(query) => {
                            let ctx = ctx.clone();
                            self.shell.set_definition(
                                kedr_workbench::lsp::DefinitionRunner::start(query, move || {
                                    ctx.request_repaint()
                                }),
                            );
                        }
                        Err(error) => self.shell.error(error),
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
                                self.shell.set_task(t);
                            }
                            Err(e) => self.shell.error(e),
                        }
                    }
                }
                Effect::RestoreDrafts(id) => {
                    let result = self
                        .shell
                        .workspace
                        .as_ref()
                        .ok_or_else(|| "Open the original workspace first".to_string())
                        .and_then(|w| {
                            self.recovery_restore =
                                Some(kedr_workbench::recovery::Guard::capture(w));
                            self.recovery
                                .as_ref()
                                .ok_or_else(|| "Recovery is unavailable in this mode".to_string())?
                                .send
                                .try_send(recovery::Request::Restore(id, w.project.root.clone()))
                                .map_err(|e| e.to_string())
                        });
                    if let Err(e) = result {
                        self.recovery_restore = None;
                        self.shell.recovery_busy = false;
                        self.shell.error(e);
                    }
                }
                Effect::ForgetDrafts(id) => {
                    let result = self
                        .recovery
                        .as_ref()
                        .ok_or_else(|| "Recovery is unavailable in this mode".to_string())
                        .and_then(|r| {
                            r.send
                                .try_send(recovery::Request::Forget(id))
                                .map_err(|e| e.to_string())
                        });
                    if let Err(e) = result {
                        self.shell.recovery_busy = false;
                        self.shell.error(e);
                    }
                }
                Effect::Quit => {
                    self.recovery_restore = None;
                    self.finishing = self.recovery.is_some();
                    self.quit_allowed = self.recovery.is_none();
                    if self.options.capture.is_none()
                        && let Err(e) = self.preferences().write()
                    {
                        self.shell.error(format!("Preferences were not saved: {e}"));
                    }
                    if self.quit_allowed {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
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
        if self.finishing && !self.quit_allowed {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if !self.finish_sent {
                self.finish_sent = self
                    .recovery
                    .as_ref()
                    .is_some_and(|r| r.send.try_send(recovery::Request::Finish).is_ok());
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Finishing local recovery journal…");
                ui.label("Waiting for the recovery worker. No project file is being written.");
                ui.spinner();
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }

        // At most one active-file content probe queued; never read files on the UI thread.
        if self.options.capture.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_secs(2));
            if !self.disk_poll_pending
                && self.disk_poll_at.elapsed() >= std::time::Duration::from_secs(2)
            {
                self.disk_poll_at = std::time::Instant::now();
                if !self.shell.loading
                    && let Some(query) = self.shell.file_query()
                {
                    self.disk_poll_pending = self
                        .jobs
                        .send
                        .try_send(Request::Probe(
                            query,
                            self.jobs.generation.load(Ordering::Relaxed),
                        ))
                        .is_ok();
                }
            }
        }
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
            self.capture_started = Some(std::time::Instant::now());
        }
        let effects = self.shell.show(ctx);
        self.effects(effects, ctx);
        if !self.finishing
            && let Some(recovery) = self.recovery.as_ref()
            && self.recovery_at.elapsed() >= std::time::Duration::from_secs(2)
        {
            self.recovery_at = std::time::Instant::now();
            if let Some(w) = self.shell.workspace.as_ref()
                && self
                    .recovery_checkpoint
                    .as_ref()
                    .is_none_or(|g| !g.matches(w))
            {
                match kedr_workbench::recovery::Snapshot::capture(w) {
                    Ok(snapshot) => {
                        if recovery
                            .send
                            .try_send(recovery::Request::Checkpoint(snapshot))
                            .is_ok()
                        {
                            self.recovery_checkpoint =
                                Some(kedr_workbench::recovery::Guard::capture(w));
                        }
                    }
                    Err(e) => self.shell.error(e.to_string()),
                }
            }
        }

        if let Some(path) = &self.options.capture {
            if ready
                && self.state_applied
                && !self.shell.search_loading
                && self.frames > 8
                && self
                    .capture_started
                    .is_some_and(|t| t.elapsed() >= std::time::Duration::from_secs(1))
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
            if self
                .capture_started
                .is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(20))
            {
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
        .with_title("CodeCedar — Kedr")
        .with_app_id("io.github.drainedgodw.CodeCedar")
        .with_icon(eframe::icon_data::from_png_bytes(icon::PNG).expect("embedded CodeCedar icon"));
    eframe::run_native(
        "CodeCedar — Kedr",
        eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(Studio::new(cc, options)))),
    )
}
