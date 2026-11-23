//! A collapsible file tree with tri-state folder checkboxes.
//!
//! Used by the add dialog (choose files before downloading) and the details
//! panel (see per-file progress, change the selection later). Rows are painted
//! by hand so long names are always clipped to their column, and only visible
//! rows are laid out, so torrents with thousands of files stay smooth.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, Rect, Sense, Stroke, TextStyle, Ui, pos2, vec2};

use super::icons::Icon;
use super::{theme, widgets};
use crate::format;

const ROW: f32 = 28.0;
const INDENT: f32 = 18.0;

/// One file as the tree sees it.
pub struct TreeFile<'a> {
    pub path: &'a Path,
    pub size: u64,
    /// Downloaded fraction, shown as a bar when present.
    pub progress: Option<f32>,
    pub selected: bool,
}

/// What the user changed: positions in the `files` slice and their new state.
pub struct Toggle {
    pub positions: Vec<usize>,
    pub selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Check {
    On,
    Off,
    Mixed,
}

#[derive(Default)]
pub struct FileTree {
    /// Folders the user expanded or collapsed, relative to the default.
    flipped: HashSet<PathBuf>,
}

#[derive(Default)]
struct Dir {
    dirs: BTreeMap<String, Dir>,
    files: Vec<usize>,
    size: u64,
    done: f64,
    selected: usize,
    count: usize,
}

impl Dir {
    fn insert(&mut self, components: &[String], pos: usize, f: &TreeFile<'_>) {
        self.size += f.size;
        self.done += f.size as f64 * f.progress.unwrap_or(0.0) as f64;
        self.count += 1;
        self.selected += usize::from(f.selected);
        match components {
            [] | [_] => self.files.push(pos),
            [first, rest @ ..] => self
                .dirs
                .entry(first.clone())
                .or_default()
                .insert(rest, pos, f),
        }
    }

    fn check(&self) -> Check {
        match self.selected {
            0 => Check::Off,
            n if n == self.count => Check::On,
            _ => Check::Mixed,
        }
    }

    fn all_files(&self, out: &mut Vec<usize>) {
        out.extend(&self.files);
        for d in self.dirs.values() {
            d.all_files(out);
        }
    }
}

enum Row<'a> {
    Dir {
        name: &'a str,
        path: PathBuf,
        dir: &'a Dir,
        depth: usize,
        expanded: bool,
    },
    File {
        pos: usize,
        depth: usize,
    },
}

impl FileTree {
    /// Draws the tree in the remaining height of `ui`, or at most `max_height`.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        files: &[TreeFile<'_>],
        selectable: bool,
        max_height: f32,
    ) -> Option<Toggle> {
        let mut root = Dir::default();
        for (pos, f) in files.iter().enumerate() {
            let components: Vec<String> = f
                .path
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            root.insert(&components, pos, f);
        }

        // Small torrents open fully; big ones start with only the top level open.
        let expand_by_default = files.len() <= 40;
        let mut rows = Vec::new();
        self.flatten(
            &root,
            PathBuf::new(),
            0,
            expand_by_default,
            files,
            &mut rows,
        );

        let show_progress = files.iter().any(|f| f.progress.is_some());
        let mut toggle = None;
        let mut flip = None;

        egui::ScrollArea::vertical()
            .max_height(max_height)
            .auto_shrink([false, true])
            .show_rows(ui, ROW, rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for row in &rows[range] {
                    let (rect, resp) =
                        ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 4.0, theme::SURFACE_HOVER);
                    }
                    match row {
                        Row::Dir {
                            name,
                            path,
                            dir,
                            depth,
                            expanded,
                        } => {
                            let check = dir.check();
                            let progress =
                                (dir.size > 0).then(|| (dir.done / dir.size as f64) as f32);
                            let parts = paint_row(
                                ui,
                                rect,
                                *depth,
                                Some(*expanded),
                                selectable.then_some(check),
                                name,
                                true,
                                check != Check::Off,
                                dir.size,
                                show_progress.then_some(progress.unwrap_or(0.0)),
                            );
                            let check_hit = selectable
                                && parts.check.is_some_and(|r| {
                                    resp.interact_pointer_pos()
                                        .is_some_and(|p| r.expand(4.0).contains(p))
                                });
                            if resp.clicked() {
                                if check_hit {
                                    let mut positions = Vec::new();
                                    dir.all_files(&mut positions);
                                    toggle = Some(Toggle {
                                        positions,
                                        selected: check != Check::On,
                                    });
                                } else {
                                    flip = Some(path.clone());
                                }
                            }
                        }
                        Row::File { pos, depth } => {
                            let f = &files[*pos];
                            let name = f
                                .path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let check = if f.selected { Check::On } else { Check::Off };
                            paint_row(
                                ui,
                                rect,
                                *depth,
                                None,
                                selectable.then_some(check),
                                &name,
                                false,
                                f.selected,
                                f.size,
                                f.progress,
                            );
                            if selectable && resp.clicked() {
                                toggle = Some(Toggle {
                                    positions: vec![*pos],
                                    selected: !f.selected,
                                });
                            }
                            resp.on_hover_text(f.path.display().to_string());
                        }
                    }
                }
            });

        if let Some(path) = flip
            && !self.flipped.remove(&path)
        {
            self.flipped.insert(path);
        }
        toggle
    }

    fn flatten<'a>(
        &self,
        dir: &'a Dir,
        path: PathBuf,
        depth: usize,
        expand_by_default: bool,
        files: &[TreeFile<'_>],
        out: &mut Vec<Row<'a>>,
    ) {
        for (name, child) in &dir.dirs {
            let child_path = path.join(name);
            let default = depth == 0 || expand_by_default;
            let expanded = default != self.flipped.contains(&child_path);
            out.push(Row::Dir {
                name,
                path: child_path.clone(),
                dir: child,
                depth,
                expanded,
            });
            if expanded {
                self.flatten(child, child_path, depth + 1, expand_by_default, files, out);
            }
        }
        let mut sorted = dir.files.clone();
        sorted.sort_by_key(|&p| files[p].path.file_name().map(|n| n.to_ascii_lowercase()));
        out.extend(sorted.into_iter().map(|pos| Row::File { pos, depth }));
    }
}

struct RowParts {
    check: Option<Rect>,
}

#[allow(clippy::too_many_arguments)]
fn paint_row(
    ui: &Ui,
    rect: Rect,
    depth: usize,
    expanded: Option<bool>,
    check: Option<Check>,
    name: &str,
    is_dir: bool,
    active: bool,
    size: u64,
    progress: Option<f32>,
) -> RowParts {
    let painter = ui.painter_at(rect);
    let small = TextStyle::Small.resolve(ui.style());
    let body = TextStyle::Body.resolve(ui.style());
    let cy = rect.center().y;
    let mut x = rect.left() + 6.0 + depth as f32 * INDENT;

    // Chevron.
    if let Some(open) = expanded {
        let c = pos2(x + 6.0, cy);
        let pts = if open {
            vec![
                c + vec2(-3.5, -1.5),
                c + vec2(0.0, 2.0),
                c + vec2(3.5, -1.5),
            ]
        } else {
            vec![
                c + vec2(-1.5, -3.5),
                c + vec2(2.0, 0.0),
                c + vec2(-1.5, 3.5),
            ]
        };
        painter.add(egui::Shape::line(pts, Stroke::new(1.4, theme::TEXT_DIM)));
    }
    x += 16.0;

    // Checkbox.
    let mut check_rect = None;
    if let Some(state) = check {
        let r = Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 16.0));
        widgets::paint_check(&painter, r, state);
        check_rect = Some(r);
        x += 24.0;
    }

    // Folder icon.
    if is_dir {
        let r = Rect::from_center_size(pos2(x + 7.0, cy), vec2(14.0, 14.0));
        Icon::Folder.paint(&painter, r, theme::TEXT_FAINT);
        x += 22.0;
    }

    // Right-hand columns, laid out from the right edge.
    let mut right = rect.right() - 10.0;
    painter.text(
        pos2(right, cy),
        Align2::RIGHT_CENTER,
        format::bytes(size),
        small.clone(),
        theme::TEXT_DIM,
    );
    right -= 76.0;
    if let Some(p) = progress {
        painter.text(
            pos2(right, cy),
            Align2::RIGHT_CENTER,
            format!("{:.0}%", p * 100.0),
            small,
            theme::TEXT_DIM,
        );
        right -= 44.0;
        let bar = Rect::from_min_max(pos2(right - 80.0, cy - 2.0), pos2(right, cy + 2.0));
        let color = if !active {
            theme::GREY
        } else if p >= 0.999 {
            theme::BLUE
        } else {
            theme::ACCENT
        };
        widgets::paint_progress(&painter, bar, p, color);
        right -= 96.0;
    }

    // Name, truncated to whatever space is left.
    let color = if active {
        theme::TEXT
    } else {
        theme::TEXT_FAINT
    };
    let width = (right - x - 8.0).max(20.0);
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_owned(), body, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    painter.galley(pos2(x, cy - galley.size().y / 2.0), galley, color);

    RowParts { check: check_rect }
}
