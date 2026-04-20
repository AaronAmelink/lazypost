use std::collections::HashSet;
use crate::helpers::items::Item;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use crate::helpers::workspace_config::{WorkspaceConfig};

impl Item {
    fn render_at(
        &self,
        area: Rect,
        buf: &mut Buffer,
        is_selected: bool,
        selected_child_path: &[usize],
        depth: u16,
        open_folders: &HashSet<Vec<usize>>,
        current_path: &[usize],
    ) -> u16 {
        let x = area.x + depth * 2;

        match self {
            Item::Request(req) => {
                let line_str = format!("  {} ", req.name);
                let line_len = line_str.len() as u16;

                let (text_style, method_style) = if is_selected {
                    (
                        Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
                        Style::default().fg(req.request_type.color()).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(Color::White),
                        Style::default().fg(req.request_type.color()),
                    )
                };

                buf.set_string(x, area.y, &line_str, text_style);
                buf.set_string(x + line_len, area.y, format!("[{}]", req.request_type.as_str()), method_style);

                1
            }

            Item::Folder(folder) => {
                let is_open = open_folders.contains(&current_path.to_vec());
                let prefix = if is_open { "v /" } else { "> /" };
                let line_str = format!("{}{}", prefix, folder.name);

                let style = if is_selected && selected_child_path.is_empty() {
                    Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                buf.set_string(x, area.y, &line_str, style);

                let mut rows = 1u16;

                if is_open {
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

                        let child_selected = selected_child_path.first() == Some(&i);
                        let grandchild_path = if child_selected { &selected_child_path[1..] } else { &[] };

                        let mut child_path = current_path.to_vec();
                        child_path.push(i);

                        rows += child.render_at(
                            child_area,
                            buf,
                            child_selected && selected_child_path.len() == 1,
                            grandchild_path,
                            depth + 1,
                            open_folders,
                            &child_path,
                        );
                    }
                }

                rows
            }
        }
    }
}

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

    pub fn select_next(&mut self) {
        let flat = self.flatten_visible();
        if let Some(pos) = flat.iter().position(|p| p == &self.selected_path) {
            if pos + 1 < flat.len() {
                self.selected_path = flat[pos + 1].clone();
            }
        }
    }

    pub fn select_prev(&mut self) {
        let flat = self.flatten_visible();
        if let Some(pos) = flat.iter().position(|p| p == &self.selected_path) {
            if pos > 0 {
                self.selected_path = flat[pos - 1].clone();
            }
        }
    }

    pub fn toggle_selected(&mut self) {
        let path = self.selected_path.clone();
        if self.item_at(&path).map(|i| matches!(i, Item::Folder(_))).unwrap_or(false) {
            if self.open_folders.contains(&path) {
                self.open_folders.remove(&path);
            } else {
                self.open_folders.insert(path);
            }
        }
    }

    pub fn add_item(&mut self, path: Vec<usize>, item: Item) -> Result<(), &'static str> {
        if path.is_empty() {
            self.items.push(item);
            return Ok(());
        }

        if self.item_at(&path).map(|i| matches!(i, Item::Folder(_))).unwrap_or(false) {
            let siblings = Self::children_at(&mut self.items, &path)?;
            siblings.push(item);
            return Ok(());
        }

        let insert_at = path.last().unwrap() + 1;
        let mut parent_path = path;
        parent_path.pop();
        let siblings = Self::children_at(&mut self.items, &parent_path)?;
        siblings.insert(insert_at.min(siblings.len()), item);
        Ok(())
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

    fn children_at<'a>(items: &'a mut Vec<Item>, path: &[usize]) -> Result<&'a mut Vec<Item>, &'static str> {
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

    fn flatten_items(items: &[Item], out: &mut Vec<Vec<usize>>, prefix: &[usize], open_folders: &HashSet<Vec<usize>>) {
        for (i, item) in items.iter().enumerate() {
            let mut path = prefix.to_vec();
            path.push(i);
            out.push(path.clone());
            if let Item::Folder(f) = item {
                if open_folders.contains(&path) {
                    Self::flatten_items(&f.items, out, &path, open_folders);
                }
            }
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

            let is_selected = self.selected_path.first() == Some(&i) && self.selected_path.len() == 1;
            let child_path = if self.selected_path.first() == Some(&i) { &self.selected_path[1..] } else { &[] };

            let rows = item.render_at(
                Rect { x: area.x, y, width: area.width, height: area.height - (y - area.y) },
                buf,
                is_selected,
                child_path,
                0,
                &self.open_folders,
                &[i],
            );
            y += rows;
        }
    }
}
