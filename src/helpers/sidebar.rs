use crate::helpers::items::Item;
use crate::helpers::workspace_config::WorkspaceConfig;
use crate::{CONFIG_PATH, helpers::items::Request};
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

                for (i, child) in folder.items.iter().enumerate() {
                    let child_area = Rect {
                        x: area.x,
                        y: area.y + rows,
                        width: area.width,
                        height: area.height.saturating_sub(rows),
                    };
                    if child_area.height == 0 {
                        break;
                    }

                    let child_selected = ctx.selected_child_path.first() == Some(&i);
                    let grandchild_path = if child_selected {
                        &ctx.selected_child_path[1..]
                    } else {
                        &[][..]
                    };

                    let mut child_path = ctx.current_path.to_vec();
                    child_path.push(i);

                    rows += child.render_at(
                        child_area,
                        buf,
                        RenderCtx {
                            is_selected: child_selected && ctx.selected_child_path.len() == 1,
                            selected_child_path: grandchild_path,
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

/// Tree of requests and folders, rendered on the left pane.
#[derive(Debug, Clone, PartialEq)]
pub struct Sidebar {
    pub selected_path: Vec<usize>,
    pub open_folders: HashSet<Vec<usize>>,
    pub items: Vec<Item>,
}

impl Sidebar {
    pub fn new(items: Vec<Item>) -> Self {
        Self {
            selected_path: vec![0],
            open_folders: HashSet::new(),
            items,
        }
    }

    /// Handles a key event when the sidebar pane has focus. Non-key or non-press
    /// events are quietly ignored. Returns `Ok(true)` unconditionally for now.
    pub fn handle_event(&mut self, event: &Event) -> Result<bool, &'static str> {
        let Event::Key(key) = event else {
            return Ok(true);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('j') => self.select_next(),
            KeyCode::Char('k') => self.select_prev(),
            KeyCode::Enter => self.toggle_selected(),
            KeyCode::Char('d') if self.remove_selected().is_ok() => {
                let _ =
                    WorkspaceConfig::save_items_to_file(self.items.clone(), Path::new(CONFIG_PATH));
            }
            _ => {}
        }
        Ok(true)
    }

    pub fn select_next(&mut self) {
        let flat = self.flatten_visible();
        if let Some(pos) = flat.iter().position(|p| p == &self.selected_path)
            && pos + 1 < flat.len()
        {
            self.selected_path = flat[pos + 1].clone();
        }
    }

    pub fn select_prev(&mut self) {
        let flat = self.flatten_visible();
        if let Some(pos) = flat.iter().position(|p| p == &self.selected_path)
            && pos > 0
        {
            self.selected_path = flat[pos - 1].clone();
        }
    }

    pub fn toggle_selected(&mut self) {
        let path = self.selected_path.clone();
        let is_folder = self
            .item_at(&path)
            .map(|i| matches!(i, Item::Folder(_)))
            .unwrap_or(false);
        if !is_folder {
            return;
        }
        if self.open_folders.contains(&path) {
            self.open_folders.remove(&path);
        } else {
            self.open_folders.insert(path);
        }
    }

    /// Inserts an item into the tree.
    ///
    /// If `path` points to a folder, the item is appended inside it; otherwise
    /// it's inserted as the next sibling of `path`. Returns the new item's
    /// path on success.
    pub fn add_item(&mut self, path: Vec<usize>, item: Item) -> Result<Vec<usize>, &'static str> {
        if path.is_empty() {
            self.items.push(item);
            return Ok(vec![self.items.len() - 1]);
        }

        let into_folder = self
            .item_at(&path)
            .map(|i| matches!(i, Item::Folder(_)))
            .unwrap_or(false);

        if into_folder {
            let siblings = Self::children_at(&mut self.items, &path)?;
            siblings.push(item);
            let mut new_path = path.clone();
            new_path.push(siblings.len() - 1);
            return Ok(new_path);
        }

        let insert_at = path.last().copied().ok_or("empty path")? + 1;
        let mut parent_path = path;
        parent_path.pop();
        let siblings = Self::children_at(&mut self.items, &parent_path)?;
        let final_index = insert_at.min(siblings.len());
        siblings.insert(final_index, item);
        let mut new_path = parent_path;
        new_path.push(final_index);
        Ok(new_path)
    }

    pub fn remove_selected(&mut self) -> Result<(), &'static str> {
        let path = self.selected_path.clone();
        self.select_prev();
        WorkspaceConfig::remove_from_items(&mut self.items, &path)
    }

    pub fn item_at(&self, path: &[usize]) -> Option<&Item> {
        Self::item_in(&self.items, path)
    }

    fn item_in<'a>(items: &'a [Item], path: &[usize]) -> Option<&'a Item> {
        let item = items.get(*path.first()?)?;
        if path.len() == 1 {
            Some(item)
        } else {
            match item {
                Item::Folder(f) => Self::item_in(&f.items, &path[1..]),
                _ => None,
            }
        }
    }

    fn children_at<'a>(
        items: &'a mut Vec<Item>,
        path: &[usize],
    ) -> Result<&'a mut Vec<Item>, &'static str> {
        if path.is_empty() {
            return Ok(items);
        }
        match items.get_mut(path[0]) {
            Some(Item::Folder(f)) => Self::children_at(&mut f.items, &path[1..]),
            Some(Item::Request(_)) => Err("Not a folder"),
            None => Err("Path out of bounds"),
        }
    }

    fn flatten_visible(&self) -> Vec<Vec<usize>> {
        let mut out = vec![];
        Self::flatten_items(&self.items, &mut out, &[], &self.open_folders);
        out
    }

    fn flatten_items(
        items: &[Item],
        out: &mut Vec<Vec<usize>>,
        prefix: &[usize],
        open_folders: &HashSet<Vec<usize>>,
    ) {
        for (i, item) in items.iter().enumerate() {
            let mut path = prefix.to_vec();
            path.push(i);
            out.push(path.clone());
            if let Item::Folder(f) = item
                && open_folders.contains(&path)
            {
                Self::flatten_items(&f.items, out, &path, open_folders);
            }
        }
    }
}

/// Replaces the request at `path` in `items` with `new_req`.
pub fn replace_request_at(
    items: &mut [Item],
    path: &[usize],
    new_req: Request,
) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("empty path");
    }
    let idx = path[0];
    if idx >= items.len() {
        return Err("out of bounds");
    }
    if path.len() == 1 {
        match &mut items[idx] {
            Item::Request(r) => {
                *r = new_req;
                Ok(())
            }
            _ => Err("not a request"),
        }
    } else {
        match &mut items[idx] {
            Item::Folder(f) => replace_request_at(&mut f.items, &path[1..], new_req),
            _ => Err("not a folder"),
        }
    }
}

impl Widget for Sidebar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        let mut y = area.y;
        for (i, item) in self.items.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }

            let is_selected =
                self.selected_path.first() == Some(&i) && self.selected_path.len() == 1;
            let child_path = if self.selected_path.first() == Some(&i) {
                &self.selected_path[1..]
            } else {
                &[][..]
            };

            let rows = item.render_at(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: area.height - (y - area.y),
                },
                buf,
                RenderCtx {
                    is_selected,
                    selected_child_path: child_path,
                    depth: 0,
                    open_folders: &self.open_folders,
                    current_path: &[i],
                },
            );
            y += rows;
        }
    }
}
