use crate::model::items::Item;
use crate::config::workspace::WorkspaceConfig;
use crate::{CONFIG_PATH, model::items::Request};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::{collections::HashSet, path::Path};

/// Per-row context for the recursive sidebar render. Carrying this struct
/// avoids an 8-argument render method.
struct RenderCtx<'a> {
    is_selected: bool,
    selected_child_path: &'a [usize],
    depth: u16,
    open_folders: &'a HashSet<Vec<usize>>,
    current_path: &'a [usize],
}

impl Item {
    /// Renders this item at the top of `area`. Returns the number of rows
    /// consumed (1 for a request; 1 + nested children if the folder is open).
    fn render_at(&self, area: Rect, buf: &mut Buffer, ctx: RenderCtx<'_>) -> u16 {
        let x = area.x + ctx.depth * 2;

        match self {
            Item::Request(req) => {
                let line_str = format!("  {} ", req.name);
                let line_len = line_str.len() as u16;

                let (text_style, method_style) = if ctx.is_selected {
                    (
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(req.request_type.color())
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(Color::White),
                        Style::default().fg(req.request_type.color()),
                    )
                };

                buf.set_string(x, area.y, &line_str, text_style);
                buf.set_string(
                    x + line_len,
                    area.y,
                    format!("[{}]", req.request_type.as_str()),
                    method_style,
                );

                1
            }

            Item::Folder(folder) => {
                let is_open = ctx.open_folders.contains(&ctx.current_path.to_vec());
                let prefix = if is_open { "v /" } else { "> /" };
                let line_str = format!("{}{}", prefix, folder.name);

                let style = if ctx.is_selected && ctx.selected_child_path.is_empty() {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                buf.set_string(x, area.y, &line_str, style);

                let mut rows = 1u16;
                if !is_open {
                    return rows;
                }

                let child_selected = ctx.is_selected && !ctx.selected_child_path.is_empty();
                for (ci, child) in folder.items.iter().enumerate() {
                    let child_area = Rect {
                        y: area.y + rows,
                        height: area.height.saturating_sub(rows),
                        ..area
                    };
                    if child_area.height == 0 {
                        break;
                    }

                    let child_path = {
                        let mut p = ctx.current_path.to_vec();
                        p.push(ci);
                        p
                    };
                    let child_selected_child = if child_selected
                        && ctx.selected_child_path[0] == ci
                    {
                        &ctx.selected_child_path[1..]
                    } else {
                        &[]
                    };

                    rows += child.render_at(
                        child_area,
                        buf,
                        RenderCtx {
                            is_selected: child_selected && ctx.selected_child_path[0] == ci,
                            selected_child_path: child_selected_child,
                            depth: ctx.depth + 1,
                            open_folders: ctx.open_folders,
                            current_path: &child_path,
                        },
                    );
                }
                rows
            }
        }
    }
}

#[derive(Clone)]
pub struct Sidebar {
    pub items: Vec<Item>,
    pub selected_path: Vec<usize>,
    open_folders: HashSet<Vec<usize>>,
    pub clipboard: Option<Item>,
}

impl Sidebar {
    pub fn new(items: Vec<Item>) -> Self {
        let selected_path = if items.is_empty() { vec![] } else { vec![0] };
        Self {
            items,
            selected_path,
            open_folders: HashSet::new(),
            clipboard: None,
        }
    }

    pub fn item_at(&self, path: &[usize]) -> Option<&Item> {
        item_at_path(&self.items, path)
    }

    pub fn add_item(
        &mut self,
        near_path: Vec<usize>,
        item: Item,
    ) -> Result<Vec<usize>, &'static str> {
        if matches!(self.item_at(&near_path), Some(Item::Folder(_))) {
            self.open_folders.insert(near_path.clone());
            let new_path = insert_into_folder(&mut self.items, &near_path, item)?;
            Ok(new_path)
        } else {
            let new_path = insert_after(&mut self.items, &near_path, item)?;
            Ok(new_path)
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Result<bool, &'static str> {
        let Event::Key(key) = event else {
            return Ok(true);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(next) = next_path(&self.items, &self.selected_path, &self.open_folders) {
                    self.selected_path = next;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(prev) = prev_path(&self.items, &self.selected_path, &self.open_folders) {
                    self.selected_path = prev;
                }
            }
            KeyCode::Enter => {
                // Toggle folder open/closed.
                if let Some(Item::Folder(_)) = self.item_at(&self.selected_path) {
                    if self.open_folders.contains(&self.selected_path) {
                        self.open_folders.remove(&self.selected_path);
                    } else {
                        self.open_folders.insert(self.selected_path.clone());
                    }
                }
            }
            KeyCode::Char('d') => {
                let path = self.selected_path.clone();
                if !path.is_empty() {
                    let flat_pos = flat_position(&self.items, &path, &self.open_folders);
                    if WorkspaceConfig::remove_from_items(&mut self.items, &path).is_ok() {
                        let _ = WorkspaceConfig::save_items_to_file(
                            self.items.clone(),
                            Path::new(CONFIG_PATH),
                        );
                        self.selected_path = path_at_flat(&self.items, flat_pos, &self.open_folders);
                    }
                }
            }
            // Cut selected item into clipboard.
            KeyCode::Char('x') => {
                let path = self.selected_path.clone();
                if !path.is_empty() {
                    let flat_pos = flat_position(&self.items, &path, &self.open_folders);
                    if let Ok(item) = WorkspaceConfig::remove_from_items(&mut self.items, &path) {
                        self.clipboard = Some(item);
                        let _ = WorkspaceConfig::save_items_to_file(
                            self.items.clone(),
                            Path::new(CONFIG_PATH),
                        );
                        self.selected_path = path_at_flat(&self.items, flat_pos, &self.open_folders);
                    }
                }
            }
            // Paste clipboard after/inside the current selection.
            KeyCode::Char('p') => {
                if let Some(item) = self.clipboard.take() {
                    let near = self.selected_path.clone();
                    let target_is_folder = matches!(self.item_at(&near), Some(Item::Folder(_)));
                    let result = if target_is_folder {
                        self.open_folders.insert(near.clone());
                        insert_into_folder(&mut self.items, &near, item)
                    } else {
                        insert_after(&mut self.items, &near, item)
                    };
                    if let Ok(new_path) = result {
                        self.selected_path = new_path;
                        let _ = WorkspaceConfig::save_items_to_file(
                            self.items.clone(),
                            Path::new(CONFIG_PATH),
                        );
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }
}

impl Widget for Sidebar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = 0u16;
        for (i, item) in self.items.iter().enumerate() {
            let item_area = Rect {
                y: area.y + y,
                height: area.height.saturating_sub(y),
                ..area
            };
            if item_area.height == 0 {
                break;
            }
            let is_selected = !self.selected_path.is_empty() && self.selected_path[0] == i;
            let selected_child_path = if is_selected && self.selected_path.len() > 1 {
                &self.selected_path[1..]
            } else {
                &[]
            };
            y += item.render_at(
                item_area,
                buf,
                RenderCtx {
                    is_selected,
                    selected_child_path,
                    depth: 0,
                    open_folders: &self.open_folders,
                    current_path: &[i],
                },
            );
        }
    }
}

// ── path helpers ────────────────────────────────────────────────────────────

fn item_at_path<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a Item> {
    if path.is_empty() {
        return None;
    }
    let item = items.get(path[0])?;
    if path.len() == 1 {
        return Some(item);
    }
    match item {
        Item::Folder(f) => item_at_path(&f.items, &path[1..]),
        Item::Request(_) => None,
    }
}

/// Flat traversal order for visible items — respects `open_folders` so closed
/// folders don't expose their children to navigation.
fn all_paths(items: &[Item], prefix: &[usize], open_folders: &HashSet<Vec<usize>>) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(i);
        out.push(path.clone());
        if let Item::Folder(f) = item {
            if open_folders.contains(&path) {
                out.extend(all_paths(&f.items, &path, open_folders));
            }
        }
    }
    out
}

fn next_path(items: &[Item], current: &[usize], open_folders: &HashSet<Vec<usize>>) -> Option<Vec<usize>> {
    let all = all_paths(items, &[], open_folders);
    let pos = all.iter().position(|p| p == current)?;
    all.into_iter().nth(pos + 1)
}

fn prev_path(items: &[Item], current: &[usize], open_folders: &HashSet<Vec<usize>>) -> Option<Vec<usize>> {
    let all = all_paths(items, &[], open_folders);
    let pos = all.iter().position(|p| p == current)?;
    if pos == 0 {
        return None;
    }
    Some(all[pos - 1].clone())
}

/// Returns the flat visible index of `path`, or 0 if not found.
fn flat_position(items: &[Item], path: &[usize], open_folders: &HashSet<Vec<usize>>) -> usize {
    let all = all_paths(items, &[], open_folders);
    all.iter().position(|p| p == path).unwrap_or(0)
}

/// After a deletion at flat position `deleted_pos`, select the item one above
/// (clamped to the new list bounds).
fn path_at_flat(items: &[Item], deleted_pos: usize, open_folders: &HashSet<Vec<usize>>) -> Vec<usize> {
    let all = all_paths(items, &[], open_folders);
    if all.is_empty() {
        return vec![];
    }
    let target = deleted_pos.saturating_sub(1).min(all.len() - 1);
    all[target].clone()
}

/// Insert `item` after `near_path` (or inside it if it's a folder), returning
/// the path where it landed.
fn insert_after(
    items: &mut Vec<Item>,
    near_path: &[usize],
    item: Item,
) -> Result<Vec<usize>, &'static str> {
    if near_path.is_empty() {
        items.push(item);
        return Ok(vec![items.len() - 1]);
    }
    let head = near_path[0];
    if near_path.len() == 1 {
        // Insert after `head` at this level.
        let insert_at = (head + 1).min(items.len());
        items.insert(insert_at, item);
        return Ok(vec![insert_at]);
    }
    // Recurse into folder.
    match items.get_mut(head) {
        Some(Item::Folder(f)) => {
            let mut child_path = insert_after(&mut f.items, &near_path[1..], item)?;
            child_path.insert(0, head);
            Ok(child_path)
        }
        _ => Err("path points into a non-folder"),
    }
}

/// Append `item` at the end of the folder at `folder_path`, returning the new
/// item's path. Differs from `insert_after` in that hovering a folder inserts
/// *inside* it rather than after it as a sibling.
fn insert_into_folder(
    items: &mut Vec<Item>,
    folder_path: &[usize],
    item: Item,
) -> Result<Vec<usize>, &'static str> {
    if folder_path.is_empty() {
        items.push(item);
        return Ok(vec![items.len() - 1]);
    }
    let head = folder_path[0];
    if folder_path.len() == 1 {
        match items.get_mut(head) {
            Some(Item::Folder(f)) => {
                f.items.push(item);
                Ok(vec![head, f.items.len() - 1])
            }
            _ => Err("path does not point to a folder"),
        }
    } else {
        match items.get_mut(head) {
            Some(Item::Folder(f)) => {
                let mut child_path = insert_into_folder(&mut f.items, &folder_path[1..], item)?;
                child_path.insert(0, head);
                Ok(child_path)
            }
            _ => Err("path points into a non-folder"),
        }
    }
}

pub fn replace_request_at(
    items: &mut Vec<Item>,
    path: &[usize],
    req: Request,
) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("empty path");
    }
    let head = path[0];
    if path.len() == 1 {
        match items.get_mut(head) {
            Some(slot @ Item::Request(_)) => {
                *slot = Item::Request(req);
                Ok(())
            }
            _ => Err("path does not point to a request"),
        }
    } else {
        match items.get_mut(head) {
            Some(Item::Folder(f)) => replace_request_at(&mut f.items, &path[1..], req),
            _ => Err("path points into a non-folder"),
        }
    }
}
