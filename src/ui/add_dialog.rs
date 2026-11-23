//! The "Add torrent" dialog: pick a source, preview files, choose a folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, RichText, Sense, Ui, vec2};

use super::icons::Icon;
use super::{App, is_link, theme, widgets};
use crate::engine::{AddRequest, Engine, Preview, Source};
use crate::format;
use crate::settings::Settings;

pub struct AddDialog {
    input: String,
    source: Option<Source>,
    request: Option<u64>,
    started: Instant,
    preview: Option<Arc<Preview>>,
    preview_error: Option<String>,
    /// Indexed like `preview.files`.
    selected: Vec<bool>,
    folder: PathBuf,
    start: bool,
    input_error: bool,
    focus_input: bool,
}

impl AddDialog {
    pub fn empty(settings: &Settings) -> Self {
        Self {
            input: String::new(),
            source: None,
            request: None,
            started: Instant::now(),
            preview: None,
            preview_error: None,
            selected: Vec::new(),
            folder: settings.download_dir.clone(),
            start: settings.start_on_add,
            input_error: false,
            focus_input: true,
        }
    }

    pub fn with_source(engine: &Engine, settings: &Settings, source: Source) -> Self {
        let mut d = Self::empty(settings);
        d.focus_input = false;
        if let Source::Url(url) = &source {
            d.input = url.clone();
        }
        d.set_source(engine, source);
        d
    }

    fn set_source(&mut self, engine: &Engine, source: Source) {
        self.preview = None;
        self.preview_error = None;
        self.selected.clear();
        self.input_error = false;
        self.started = Instant::now();
        self.request = Some(engine.request_preview(source.clone()));
        self.source = Some(source);
    }

    pub fn is_resolving(&self) -> bool {
        self.source.is_some() && self.preview.is_none() && self.preview_error.is_none()
    }

    pub fn on_preview(&mut self, request: u64, result: Result<Arc<Preview>, String>) {
        if self.request != Some(request) {
            return; // A stale answer for a source the user already replaced.
        }
        match result {
            Ok(preview) => {
                self.selected = vec![true; preview.files.len()];
                self.preview = Some(preview);
            }
            Err(e) => self.preview_error = Some(e),
        }
    }

    fn selection(&self) -> (usize, u64) {
        let Some(p) = &self.preview else {
            return (0, 0);
        };
        p.files
            .iter()
            .zip(&self.selected)
            .filter(|(_, s)| **s)
            .fold((0, 0), |(n, size), (f, _)| (n + 1, size + f.size))
    }

    fn request(&self) -> Option<AddRequest> {
        let source = self.source.clone()?;
        let only_files = self.preview.as_ref().and_then(|p| {
            let all = self.selected.iter().all(|s| *s);
            (!all).then(|| {
                p.files
                    .iter()
                    .zip(&self.selected)
                    .filter(|(_, s)| **s)
                    .map(|(f, _)| f.index)
                    .collect()
            })
        });
        Some(AddRequest {
            source,
            preview: self.preview.clone(),
            folder: Some(self.folder.clone()),
            only_files,
            paused: !self.start,
        })
    }
}

impl App {
    pub(super) fn add_modal(&mut self, ctx: &egui::Context) {
        let s = self.s();
        let already_added = self
            .add_dialog
            .as_ref()
            .and_then(|d| d.preview.as_ref())
            .is_some_and(|p| self.torrents.iter().any(|t| t.info_hash == p.info_hash));
        let Some(engine) = &self.engine else { return };
        let Some(dialog) = &mut self.add_dialog else {
            return;
        };

        let mut close = false;
        let mut submit = false;

        let modal = egui::Modal::new(egui::Id::unique("add")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.heading(s.add_torrent);
            ui.add_space(14.0);

            // Source row: magnet input + file picker.
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut dialog.input)
                    .hint_text(RichText::new(s.magnet_placeholder).color(theme::TEXT_FAINT))
                    .desired_width(ui.available_width() - 44.0)
                    .margin(vec2(10.0, 8.0));
                let resp = ui.add(edit);
                if dialog.focus_input {
                    resp.request_focus();
                    dialog.focus_input = false;
                }
                if resp.changed() {
                    let text = dialog.input.trim().to_owned();
                    if text.is_empty() {
                        dialog.input_error = false;
                    } else if is_link(&text) {
                        dialog.set_source(engine, Source::Url(text));
                    } else {
                        dialog.input_error = true;
                    }
                }
                if widgets::icon_button(ui, Icon::File, s.choose_file, true).clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Torrent", &["torrent"])
                        .pick_file()
                {
                    match Source::from_path(&path) {
                        Ok(source) => {
                            dialog.input = path.display().to_string();
                            dialog.set_source(engine, source);
                        }
                        Err(e) => dialog.preview_error = Some(format!("{e:#}")),
                    }
                }
            });
            if dialog.input_error {
                ui.label(RichText::new(s.invalid_source).small().color(theme::RED));
            }

            if let Some(source) = &dialog.source {
                ui.add_space(16.0);
                preview_section(ui, dialog_preview_state(dialog, source), dialog, s);
                if already_added {
                    ui.label(RichText::new(s.already_added).small().color(theme::AMBER));
                }
            }
            App::add_dialog_files(ui, dialog, s);

            ui.add_space(16.0);
            widgets::section_title(ui, s.save_to);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let frame = egui::Frame::new()
                    .fill(theme::INPUT_BG)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .corner_radius(theme::RADIUS)
                    .inner_margin(egui::Margin::symmetric(10, 7));
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width() - 100.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(dialog.folder.display().to_string())
                                .color(theme::TEXT_DIM),
                        )
                        .truncate(),
                    );
                });
                if widgets::button(ui, Some(Icon::Folder), s.change, false).clicked()
                    && let Some(dir) = rfd::FileDialog::new()
                        .set_directory(&dialog.folder)
                        .pick_folder()
                {
                    dialog.folder = dir;
                }
            });

            ui.add_space(14.0);
            widgets::checkbox(ui, &mut dialog.start, s.start_now);

            ui.add_space(20.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (count, _) = dialog.selection();
                let ok = dialog.source.is_some()
                    && (dialog.preview.is_none() || count > 0)
                    && dialog.preview_error.is_none()
                    && !already_added;
                let enter = ok && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if widgets::button_enabled(ui, None, s.add, true, ok).clicked() || enter {
                    submit = true;
                }
                if widgets::button(ui, None, s.cancel, false).clicked() {
                    close = true;
                }
            });
        });

        if modal.should_close() {
            close = true;
        }
        if submit {
            if let Some(req) = dialog.request() {
                engine.add(req);
            }
            close = true;
        }
        if close {
            self.add_dialog = None;
            self.refresh(true);
        }
    }
}

enum PreviewState<'a> {
    Loading { name: String, secs: f32 },
    Ready(&'a Preview),
    Failed(&'a str),
}

fn dialog_preview_state<'a>(dialog: &'a AddDialog, source: &Source) -> PreviewState<'a> {
    if let Some(p) = &dialog.preview {
        PreviewState::Ready(p)
    } else if let Some(e) = &dialog.preview_error {
        PreviewState::Failed(e)
    } else {
        PreviewState::Loading {
            name: source.display_name(),
            secs: dialog.started.elapsed().as_secs_f32(),
        }
    }
}

fn preview_section(
    ui: &mut Ui,
    state: PreviewState<'_>,
    dialog: &AddDialog,
    s: &crate::i18n::Strings,
) {
    match state {
        PreviewState::Failed(error) => {
            ui.label(RichText::new(error).color(theme::RED));
        }
        PreviewState::Loading { name, secs } => {
            ui.label(RichText::new(name).color(theme::TEXT).strong());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(14.0).color(theme::TEXT_DIM));
                ui.label(RichText::new(s.resolving).color(theme::TEXT_DIM));
            });
            if secs > 4.0 {
                ui.label(
                    RichText::new(s.resolving_hint)
                        .small()
                        .color(theme::TEXT_FAINT),
                );
            }
        }
        PreviewState::Ready(p) => {
            ui.label(RichText::new(&p.name).color(theme::TEXT).strong());
            ui.add_space(2.0);
            let (count, size) = dialog.selection();
            ui.label(
                RichText::new((s.selection_summary)(
                    count,
                    p.files.len(),
                    &format::bytes(size),
                ))
                .small()
                .color(theme::TEXT_DIM),
            );
        }
    }
}

/// The file list lives outside `preview_section` because it needs `&mut` access.
impl App {
    pub(super) fn add_dialog_files(ui: &mut Ui, dialog: &mut AddDialog, s: &crate::i18n::Strings) {
        let Some(p) = dialog.preview.clone() else {
            return;
        };
        if p.files.len() <= 1 {
            return;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.link(RichText::new(s.select_all).small()).clicked() {
                dialog.selected.iter_mut().for_each(|x| *x = true);
            }
            if ui.link(RichText::new(s.select_none).small()).clicked() {
                dialog.selected.iter_mut().for_each(|x| *x = false);
            }
        });
        ui.add_space(4.0);
        egui::Frame::new()
            .fill(theme::INPUT_BG)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(theme::RADIUS)
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (i, f) in p.files.iter().enumerate() {
                            let (rect, resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 28.0),
                                Sense::click(),
                            );
                            if resp.hovered() {
                                ui.painter().rect_filled(rect, 4.0, theme::SURFACE_HOVER);
                            }
                            if resp.clicked() {
                                dialog.selected[i] = !dialog.selected[i];
                            }
                            let mut child = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect.shrink2(vec2(6.0, 0.0)))
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            widgets::checkbox(&mut child, &mut dialog.selected[i], "");
                            let size = format::bytes(f.size);
                            let name_w = child.available_width() - 80.0;
                            widgets::path_label(&mut child, &f.path, dialog.selected[i], name_w);
                            child.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(RichText::new(size).small().color(theme::TEXT_DIM));
                                },
                            );
                        }
                    });
            });
    }
}
