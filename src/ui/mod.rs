//! The application window.

mod add_dialog;
#[cfg(test)]
mod demo;
mod details;
mod file_tree;
mod icons;
mod list;
mod settings_view;
mod theme;
mod toasts;
mod widgets;

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Frame, Key, Margin, Modifiers, Panel, RichText, Stroke, vec2};

use crate::engine::{
    Engine, Event, FileView, PeerView, PendingView, Source, Status, TorrentId, TorrentView,
};
use crate::format;
use crate::i18n::Strings;
use crate::platform;
use crate::settings::{Paths, Settings};

use add_dialog::AddDialog;
use icons::Icon;
use settings_view::SettingsView;
use toasts::Toasts;

/// How often torrent stats are re-read while the window is idle.
const REFRESH: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Downloading,
    Seeding,
    Completed,
    Paused,
    Errors,
}

impl Filter {
    const ALL: [Filter; 6] = [
        Filter::All,
        Filter::Downloading,
        Filter::Seeding,
        Filter::Completed,
        Filter::Paused,
        Filter::Errors,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            Filter::All => s.all,
            Filter::Downloading => s.downloading,
            Filter::Seeding => s.seeding,
            Filter::Completed => s.completed,
            Filter::Paused => s.paused,
            Filter::Errors => s.errors,
        }
    }

    fn matches(self, t: &TorrentView) -> bool {
        match self {
            Filter::All => true,
            Filter::Downloading => matches!(t.status, Status::Downloading | Status::Checking),
            Filter::Seeding => t.status == Status::Seeding,
            Filter::Completed => matches!(t.status, Status::Seeding | Status::Done),
            Filter::Paused => t.status == Status::Paused,
            Filter::Errors => t.status == Status::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Added,
    Name,
    Size,
    Down,
    Up,
    Eta,
    Peers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetailsTab {
    General,
    Files,
    Peers,
}

struct RemoveDialog {
    ids: Vec<TorrentId>,
    delete_files: bool,
}

/// Per-torrent data that is only needed for the details panel, refreshed less often.
#[derive(Default)]
struct DetailCache {
    id: Option<TorrentId>,
    files: Vec<FileView>,
    peers: Vec<PeerView>,
    refreshed: Option<Instant>,
    tree: file_tree::FileTree,
}

pub struct App {
    engine: Option<Engine>,
    paths: Paths,
    settings: Settings,

    torrents: Vec<TorrentView>,
    pending: Vec<PendingView>,
    speed: (u64, u64),
    last_refresh: Option<Instant>,
    last_status: HashMap<TorrentId, Status>,

    filter: Filter,
    search: String,
    sort: SortKey,
    sort_desc: bool,
    selected: BTreeSet<TorrentId>,
    anchor: Option<TorrentId>,

    details_tab: DetailsTab,
    detail: DetailCache,

    add_dialog: Option<AddDialog>,
    remove_dialog: Option<RemoveDialog>,
    settings_view: Option<SettingsView>,
    toasts: Toasts,
    fatal: Option<String>,
    title: String,
    focus_search: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths, initial: Vec<Source>) -> Self {
        theme::apply(&cc.egui_ctx);
        let settings = Settings::load(&paths.config_file);

        let ctx = cc.egui_ctx.clone();
        let notify = Arc::new(move || ctx.request_repaint());
        let (engine, fatal) = match Engine::start(&settings, paths.session_dir.clone(), notify) {
            Ok(engine) => (Some(engine), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };

        let mut app = Self::with_engine(engine, fatal, paths, settings);
        for source in initial {
            app.open_source(source);
        }
        app
    }

    fn with_engine(
        engine: Option<Engine>,
        fatal: Option<String>,
        paths: Paths,
        settings: Settings,
    ) -> Self {
        Self {
            engine,
            paths,
            settings,
            torrents: Vec::new(),
            pending: Vec::new(),
            speed: (0, 0),
            last_refresh: None,
            last_status: HashMap::new(),
            filter: Filter::All,
            search: String::new(),
            sort: SortKey::Added,
            sort_desc: true,
            selected: BTreeSet::new(),
            anchor: None,
            details_tab: DetailsTab::General,
            detail: DetailCache::default(),
            add_dialog: None,
            remove_dialog: None,
            settings_view: None,
            toasts: Toasts::default(),
            fatal,
            title: String::new(),
            focus_search: false,
        }
    }

    fn s(&self) -> &'static Strings {
        self.settings.language.strings()
    }

    /// Opens a torrent from outside the dialog (drop, paste, command line).
    pub fn open_source(&mut self, source: Source) {
        let Some(engine) = &self.engine else { return };
        if self.add_dialog.is_some() {
            // A dialog is already open; add straight away rather than losing the request.
            engine.add(crate::engine::AddRequest {
                source,
                preview: None,
                folder: Some(self.settings.download_dir.clone()),
                only_files: None,
                paused: !self.settings.start_on_add,
            });
            return;
        }
        self.add_dialog = Some(AddDialog::with_source(engine, &self.settings, source));
    }

    fn refresh(&mut self, force: bool) {
        let Some(engine) = &self.engine else { return };
        let due = self.last_refresh.is_none_or(|t| t.elapsed() >= REFRESH);
        if !force && !due {
            return;
        }
        self.last_refresh = Some(Instant::now());
        self.torrents = engine.torrents();
        self.pending = engine.pending();
        self.speed = engine.total_speed();

        let s = self.s();
        for t in &self.torrents {
            let previous = self.last_status.insert(t.id, t.status);
            let finished = matches!(t.status, Status::Seeding | Status::Done);
            if previous == Some(Status::Downloading) && finished {
                self.toasts.success(s.finished, &t.name);
            }
        }
        let ids: BTreeSet<_> = self.torrents.iter().map(|t| t.id).collect();
        self.selected.retain(|id| ids.contains(id));
        self.last_status.retain(|id, _| ids.contains(id));

        // Details are heavier; only refresh them for the focused torrent.
        let focused = self.focused();
        let stale = self
            .detail
            .refreshed
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(1));
        if focused != self.detail.id || stale {
            if focused != self.detail.id {
                self.detail.tree = Default::default();
            }
            self.detail.id = focused;
            self.detail.refreshed = Some(Instant::now());
            if let Some(id) = focused {
                match self.details_tab {
                    DetailsTab::Files => self.detail.files = engine.files(id),
                    DetailsTab::Peers => self.detail.peers = engine.peers(id),
                    DetailsTab::General => {}
                }
            }
        }
    }

    fn handle_events(&mut self) {
        let Some(engine) = &self.engine else { return };
        let s = self.s();
        let mut changed = false;
        while let Some(event) = engine.poll_event() {
            changed = true;
            match event {
                Event::Added { name } => self.toasts.info(s.added, &name),
                Event::AlreadyAdded { name } => self.toasts.info(s.already_added, &name),
                Event::Failed { context, error } => {
                    self.toasts.error(s.failed, &format!("{context}: {error}"))
                }
                Event::Preview { request, result } => {
                    if let Some(dialog) = &mut self.add_dialog {
                        dialog.on_preview(request, result);
                    }
                }
            }
        }
        if changed {
            self.refresh(true);
        }
    }

    /// The single torrent the details panel describes.
    fn focused(&self) -> Option<TorrentId> {
        if self.selected.len() == 1 {
            self.selected.first().copied()
        } else {
            None
        }
    }

    /// Torrents after filtering and sorting, in display order.
    fn visible(&self) -> Vec<&TorrentView> {
        let needle = self.search.trim().to_lowercase();
        let mut list: Vec<&TorrentView> = self
            .torrents
            .iter()
            .filter(|t| self.filter.matches(t))
            .filter(|t| needle.is_empty() || t.name.to_lowercase().contains(&needle))
            .collect();
        list.sort_by(|a, b| {
            let ord = match self.sort {
                SortKey::Added => a.id.cmp(&b.id),
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Size => a.total_bytes.cmp(&b.total_bytes),
                SortKey::Down => a.down_speed.cmp(&b.down_speed),
                SortKey::Up => a.up_speed.cmp(&b.up_speed),
                SortKey::Eta => a
                    .eta_secs
                    .unwrap_or(u64::MAX)
                    .cmp(&b.eta_secs.unwrap_or(u64::MAX)),
                SortKey::Peers => a.peers.cmp(&b.peers),
            };
            if self.sort_desc { ord.reverse() } else { ord }
        });
        list
    }

    fn selected_views(&self) -> impl Iterator<Item = &TorrentView> {
        self.torrents
            .iter()
            .filter(|t| self.selected.contains(&t.id))
    }

    fn pause_selected(&self) {
        if let Some(engine) = &self.engine {
            for t in self.selected_views() {
                engine.pause(t.id);
            }
        }
    }

    fn resume_selected(&self) {
        if let Some(engine) = &self.engine {
            for t in self.selected_views() {
                engine.resume(t.id);
            }
        }
    }

    fn ask_remove(&mut self, delete_files: bool) {
        if !self.selected.is_empty() {
            self.remove_dialog = Some(RemoveDialog {
                ids: self.selected.iter().copied().collect(),
                delete_files,
            });
        }
    }

    fn open_folder(&mut self, t: &TorrentView) {
        let folder = if t.output_folder.exists() {
            t.output_folder.clone()
        } else {
            self.settings.download_dir.clone()
        };
        if let Err(e) = platform::open_path(&folder) {
            self.toasts.error(self.s().failed, &e.to_string());
        }
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(&self.paths.config_file) {
            tracing::warn!("could not save settings: {e:#}");
        }
    }

    fn any_dialog_open(&self) -> bool {
        self.add_dialog.is_some() || self.remove_dialog.is_some() || self.settings_view.is_some()
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        // Files dropped onto the window.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for path in dropped {
            match Source::from_path(&path) {
                Ok(source) => self.open_source(source),
                Err(e) => self.toasts.error(self.s().failed, &format!("{e:#}")),
            }
        }

        let typing = ctx.memory(|m| m.focused().is_some());
        if self.any_dialog_open() {
            return;
        }

        // Ctrl+V anywhere outside a text field: paste a magnet link.
        if !typing {
            let pasted = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Paste(text) if is_link(text) => Some(text.trim().to_owned()),
                    _ => None,
                })
            });
            if let Some(link) = pasted {
                self.open_source(Source::Url(link));
                return;
            }
        }

        let cmd = Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(cmd, Key::O))
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Torrent", &["torrent"])
                .pick_file()
        {
            match Source::from_path(&path) {
                Ok(source) => self.open_source(source),
                Err(e) => self.toasts.error(self.s().failed, &format!("{e:#}")),
            }
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::N)) {
            self.add_dialog = Some(AddDialog::empty(&self.settings));
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::F)) {
            self.focus_search = true;
        }
        if typing {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::A)) {
            let ids: Vec<_> = self.visible().iter().map(|t| t.id).collect();
            self.selected = ids.into_iter().collect();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete)) {
            self.ask_remove(false);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Delete)) {
            self.ask_remove(true);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space)) {
            let any_active = self.selected_views().any(|t| t.status.is_active());
            if any_active {
                self.pause_selected();
            } else {
                self.resume_selected();
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.selected.clear();
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let (down, up) = self.speed;
        let title = if down > 0 || up > 0 {
            format!(
                "Riptide  ↓ {}  ↑ {}",
                format::speed(down).trim_end_matches("/s"),
                format::speed(up).trim_end_matches("/s")
            )
            .replace("↑ ", if up > 0 { "↑ " } else { "" })
        } else {
            "Riptide".to_owned()
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    // ---- Panels ---------------------------------------------------------------

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let s = self.s();
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Riptide")
                    .font(theme::medium(19.0))
                    .color(theme::TEXT),
            );
        });
        ui.add_space(18.0);

        for filter in Filter::ALL {
            let count = self.torrents.iter().filter(|t| filter.matches(t)).count();
            if filter == Filter::Errors && count == 0 {
                continue;
            }
            let selected = self.filter == filter;
            let (rect, resp) =
                ui.allocate_exact_size(vec2(ui.available_width(), 32.0), egui::Sense::click());
            let painter = ui.painter();
            if selected {
                painter.rect_filled(rect, theme::RADIUS, theme::SURFACE_ACTIVE);
            } else if resp.hovered() {
                painter.rect_filled(rect, theme::RADIUS, theme::SURFACE_HOVER);
            }
            let text_color = if selected {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            };
            let font = egui::TextStyle::Body.resolve(ui.style());
            painter.text(
                rect.left_center() + vec2(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                filter.label(s),
                font,
                text_color,
            );
            if count > 0 {
                let color = if filter == Filter::Errors {
                    theme::RED
                } else {
                    theme::TEXT_FAINT
                };
                painter.text(
                    rect.right_center() - vec2(12.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    egui::TextStyle::Small.resolve(ui.style()),
                    color,
                );
            }
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.filter = filter;
            }
            ui.add_space(2.0);
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(12.0);
            if widgets::button(ui, Some(Icon::Settings), s.settings, false).clicked() {
                self.settings_view = Some(SettingsView::new(&self.settings));
            }
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let s = self.s();
        let has_selection = !self.selected.is_empty();
        let can_pause = self.selected_views().any(|t| t.status.is_active());
        let can_resume = self
            .selected_views()
            .any(|t| matches!(t.status, Status::Paused | Status::Done | Status::Error));

        ui.horizontal_centered(|ui| {
            if widgets::button(ui, Some(Icon::Plus), s.add_torrent, true).clicked() {
                self.add_dialog = Some(AddDialog::empty(&self.settings));
            }
            ui.add_space(8.0);
            if widgets::icon_button(ui, Icon::Play, s.resume, can_resume).clicked() {
                self.resume_selected();
            }
            if widgets::icon_button(ui, Icon::Pause, s.pause, can_pause).clicked() {
                self.pause_selected();
            }
            if widgets::icon_button(ui, Icon::Trash, s.remove, has_selection).clicked() {
                self.ask_remove(false);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.search_field(ui);
            });
        });
    }

    fn search_field(&mut self, ui: &mut egui::Ui) {
        let s = self.s();
        let width = 240.0;
        let frame = Frame::new()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(theme::RADIUS)
            .inner_margin(Margin::symmetric(8, 0));
        frame.show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(32.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                Icon::Search.paint(ui.painter(), r, theme::TEXT_FAINT);
                let edit = egui::TextEdit::singleline(&mut self.search)
                    .hint_text(RichText::new(s.search).color(theme::TEXT_FAINT))
                    .frame(egui::Frame::NONE)
                    .align(egui::Align2::LEFT_CENTER)
                    .min_size(vec2(0.0, 30.0))
                    .desired_width(width - 52.0);
                let resp = ui.add(edit);
                if self.focus_search {
                    resp.request_focus();
                    self.focus_search = false;
                }
                if !self.search.is_empty()
                    && widgets::icon_button(ui, Icon::Close, "", true).clicked()
                {
                    self.search.clear();
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let s = self.s();
        let (down, up) = self.speed;
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let small = |t: String| RichText::new(t).small().color(theme::TEXT_DIM);

            let limits = (
                self.settings.download_limit_kib,
                self.settings.upload_limit_kib,
            );
            for (icon, speed, limit) in [
                (Icon::ArrowDown, down, limits.0),
                (Icon::ArrowUp, up, limits.1),
            ] {
                let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), egui::Sense::hover());
                icon.paint(ui.painter(), r, theme::TEXT_FAINT);
                let mut text = if speed == 0 {
                    "0 B/s".to_owned()
                } else {
                    format::speed(speed)
                };
                if limit > 0 {
                    text.push_str(&format!(" / {}", format::speed(limit as u64 * 1024)));
                }
                ui.label(small(text));
                ui.add_space(14.0);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(port) = self.engine.as_ref().and_then(Engine::listen_port) {
                    ui.label(small(format!("{} {port}", s.port)));
                    ui.add_space(12.0);
                }
                ui.label(small((s.torrents_count)(self.torrents.len())));
            });
        });
    }

    fn remove_modal(&mut self, ctx: &egui::Context) {
        let s = self.s();
        let Some(dialog) = &mut self.remove_dialog else {
            return;
        };
        let mut close = false;
        let mut confirm = false;
        let modal = egui::Modal::new(egui::Id::unique("remove")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(s.remove_title);
            ui.add_space(10.0);
            ui.label(RichText::new((s.remove_count)(dialog.ids.len())).color(theme::TEXT_DIM));
            ui.add_space(14.0);
            widgets::checkbox(ui, &mut dialog.delete_files, s.also_delete_files);
            ui.add_space(20.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if dialog.delete_files {
                    s.remove_with_files
                } else {
                    s.remove
                };
                if danger_button(ui, label).clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                    confirm = true;
                }
                if widgets::button(ui, None, s.cancel, false).clicked() {
                    close = true;
                }
            });
        });
        if modal.should_close() {
            close = true;
        }
        if confirm {
            if let Some(engine) = &self.engine {
                for id in &dialog.ids {
                    engine.remove(*id, dialog.delete_files);
                }
            }
            self.selected.clear();
            close = true;
        }
        if close {
            self.remove_dialog = None;
        }
    }

    fn drop_overlay(&self, ctx: &egui::Context) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if !hovering {
            return;
        }
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("drop"),
        ));
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(170));
        let inner = rect.shrink(24.0);
        painter.rect_stroke(
            inner,
            12.0,
            Stroke::new(1.5, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.s().drop_to_add,
            egui::FontId::proportional(18.0),
            theme::TEXT,
        );
    }

    fn fatal_screen(&self, ui: &mut egui::Ui, error: &str) {
        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(theme::BG).inner_margin(40))
            .show(ui, |ui| {
                ui.heading("Riptide could not start");
                ui.add_space(8.0);
                ui.label(RichText::new(error).color(theme::RED));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Another copy may already be running, or the listening port is in use.",
                    )
                    .color(theme::TEXT_DIM),
                );
            });
    }
}

fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::WHITE);
    let (rect, resp) =
        ui.allocate_exact_size(vec2(galley.size().x + 28.0, 32.0), egui::Sense::click());
    let fill = if resp.hovered() {
        egui::Color32::from_rgb(0xea, 0x74, 0x6e)
    } else {
        theme::RED
    };
    ui.painter().rect_filled(rect, theme::RADIUS, fill);
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        egui::Color32::WHITE,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// True for things that can be added straight from the clipboard.
pub fn is_link(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("magnet:?")
        || ((t.starts_with("http://") || t.starts_with("https://")) && t.ends_with(".torrent"))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(error) = self.fatal.clone() {
            self.fatal_screen(ui, &error);
            return;
        }

        self.handle_events();
        self.handle_input(&ctx);
        self.refresh(false);
        self.update_title(&ctx);

        Panel::bottom("status")
            .exact_size(30.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(theme::SIDEBAR)
                    .inner_margin(Margin::symmetric(14, 0))
                    .stroke(Stroke::new(1.0, theme::BORDER)),
            )
            .show(ui, |ui| self.status_bar(ui));

        Panel::left("sidebar")
            .exact_size(196.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(theme::SIDEBAR)
                    .inner_margin(Margin::symmetric(10, 0))
                    .stroke(Stroke::new(1.0, theme::BORDER)),
            )
            .show(ui, |ui| self.sidebar(ui));

        Panel::top("toolbar")
            .exact_size(56.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| self.toolbar(ui));

        if let Some(id) = self.focused() {
            Panel::bottom("details")
                .resizable(true)
                .default_size(240.0)
                .size_range(140.0..=600.0)
                .show_separator_line(true)
                .frame(
                    Frame::new()
                        .fill(theme::SIDEBAR)
                        .inner_margin(Margin::symmetric(16, 12)),
                )
                .show(ui, |ui| self.details(ui, id));
        }

        egui::CentralPanel::no_frame()
            .frame(Frame::new().fill(theme::BG))
            .show(ui, |ui| self.list(ui));

        // Modals and overlays go last so they draw on top.
        if self.add_dialog.is_some() {
            self.add_modal(&ctx);
        }
        self.remove_modal(&ctx);
        if self.settings_view.is_some() {
            self.settings_modal(&ctx);
        }
        self.drop_overlay(&ctx);
        self.toasts.show(&ctx);

        let animating = !self.pending.is_empty()
            || self
                .add_dialog
                .as_ref()
                .is_some_and(AddDialog::is_resolving);
        ctx.request_repaint_after(if animating {
            Duration::from_millis(50)
        } else {
            REFRESH
        });
    }

    fn on_exit(&mut self) {
        self.save_settings();
        if let Some(engine) = self.engine.take() {
            engine.shutdown();
        }
    }
}
