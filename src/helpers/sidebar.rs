use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RequestType {
    Get,
    Post,
    Put,
    Delete,
}

impl RequestType {
    fn as_str(&self) -> &str {
        match self {
            RequestType::Get => "GET",
            RequestType::Post => "POST",
            RequestType::Put => "PUT",
            RequestType::Delete => "DELETE",
        }
    }

    fn color(&self) -> Color {
        match self {
            RequestType::Get => Color::Blue,
            RequestType::Post => Color::Green,
            RequestType::Put => Color::Yellow,
            RequestType::Delete => Color::Red,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HTTPSidebarItem {
    pub label: RequestType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FolderSidebarItem {
    pub items: Vec<SidebarItem>,
    pub open: bool,
}

pub trait Folder {
    fn items(&self) -> &Vec<SidebarItem>;
    fn items_mut(&mut self) -> &mut Vec<SidebarItem>;

    fn add_item(&mut self, path: Vec<usize>, item: SidebarItem) -> Result<(), &str> {
        if path.len() == 1 {
            if self.items().iter().any(|i| i.name == item.name && i.item_type == item.item_type) {
                return Err("Item with the same type and name already exists");
            }
            self.items_mut().push(item);
            Ok(())
        } else {
            if path[0] >= self.items().len() {
                return Err("Path out of bounds");
            }
            match &self.items()[path[0]].item_type {
                SidebarItemType::Folder(_) => {
                    if let SidebarItemType::Folder(folder) = &mut self.items_mut()[path[0]].item_type {
                        folder.add_item(path[1..].to_vec(), item)
                    } else {
                        unreachable!()
                    }
                }
                SidebarItemType::HTTP(_) => {
                    self.items_mut().insert(path[0] + 1, item);
                    Ok(())
                }
            }
        }
    }

    fn remove_item(&mut self, path: &[usize]) -> Result<(), &'static str> {
        if path.len() == 1 {
            self.items_mut().remove(path[0]);
            Ok(())
        } else {
            let first = path[0];
            if first >= self.items().len() {
                return Err("Path out of bounds");
            }
            if let SidebarItemType::Folder(folder) = &mut self.items_mut()[first].item_type {
                folder.remove_item(&path[1..])
            } else {
                Err("Path out of bounds")
            }
        }
    }
}

impl FolderSidebarItem {
    fn activate(&mut self) {
        self.open = !self.open;
    }

    fn get_height(&self) -> u16 {
        if self.open {
            1 + self.items().iter().map(|i| i.get_height()).sum::<u16>()
        } else {
            1
        }
    }
}

impl Folder for FolderSidebarItem {
    fn items(&self) -> &Vec<SidebarItem> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<SidebarItem> {
        &mut self.items
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarItemType {
    HTTP(HTTPSidebarItem),
    Folder(FolderSidebarItem),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarItem {
    pub name: String,
    pub item_type: SidebarItemType,
}

impl SidebarItem {
    pub fn new_folder(name: String, sub_items: Vec<SidebarItem>) -> Self {
        Self {
            name,
            item_type: SidebarItemType::Folder(FolderSidebarItem {
                items: sub_items,
                open: false,
            }),
        }
    }

    pub fn new_http(name: String, label: RequestType) -> Self {
        Self {
            name,
            item_type: SidebarItemType::HTTP(HTTPSidebarItem { label }),
        }
    }

    pub fn activate(&mut self) {
        if let SidebarItemType::Folder(ref mut folder) = self.item_type {
            folder.activate();
        }
    }

    pub fn get_height(&self) -> u16 {
        match &self.item_type {
            SidebarItemType::HTTP(_) => 1,
            SidebarItemType::Folder(f) => f.get_height(),
        }
    }

    fn render_at(&self, area: Rect, buf: &mut Buffer, is_selected: bool, selected_child_path: &[usize]) {
        match &self.item_type {
            SidebarItemType::HTTP(http) => {
                let method_str = http.label.as_str();
                let method_color = http.label.color();

                let prefix = "  ";
                let line_str = format!("{}{} ", prefix, self.name);
                let line_len = line_str.len() as u16;

                let style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                buf.set_string(area.x, area.y, line_str, style);

                let method_start = area.x + line_len + 1;
                let method_style = if is_selected {
                    Style::default()
                        .fg(method_color)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(method_color)
                };

                buf.set_string(method_start, area.y, format!("[{}]", method_str), method_style);
            }

            SidebarItemType::Folder(folder) => {
                let prefix = if folder.open {
                    "v "
                } else {
                    "> "
                };
                let line_str = format!("{}/{}", prefix, self.name);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                buf.set_string(area.x, area.y, line_str, style);

                if folder.open {
                    let mut y = area.y + 1;
                    for (i, item) in folder.items.iter().enumerate() {
                        let child_is_selected =
                            selected_child_path.first() == Some(&i)
                            && selected_child_path.len() == 1;
                        let grandchild_path = if selected_child_path.first() == Some(&i) {
                            &selected_child_path[1..]
                        } else {
                            &[]
                        };
                        let item_height = item.get_height();
                        item.render_at(
                            Rect { x: area.x + 2, y, width: area.width.saturating_sub(2), height: item_height },
                            buf,
                            child_is_selected,
                            grandchild_path,
                        );
                        y += item_height;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sidebar {
    pub selected_path: Vec<usize>,
    pub items: Vec<SidebarItem>,
}

impl Sidebar {
    pub fn new(items: Vec<SidebarItem>) -> Self {
        Self {
            selected_path: vec![0],
            items,
        }
    }

    fn visible_paths(&self) -> Vec<Vec<usize>> {
        let mut result = vec![];
        collect_visible_paths(&self.items, &[], &mut result);
        result
    }

    pub fn next(&mut self) {
        let visible = self.visible_paths();
        if let Some(pos) = visible.iter().position(|p| p == &self.selected_path) {
            if pos + 1 < visible.len() {
                self.selected_path = visible[pos + 1].clone();
            }
        }
    }

    pub fn previous(&mut self) {
        let visible = self.visible_paths();
        if let Some(pos) = visible.iter().position(|p| p == &self.selected_path) {
            if pos > 0 {
                self.selected_path = visible[pos - 1].clone();
            }
        }
    }

    pub fn activate(&mut self) {
        activate_at(&mut self.items, &self.selected_path);
    }

    pub fn remove_selected(&mut self) -> Result<(), &'static str> {
        let path = self.selected_path.clone();
        self.remove_item(&path)?;
        self.fix_selected_path();
        Ok(())
    }

    fn fix_selected_path(&mut self) {
        let mut items: &[SidebarItem] = &self.items;
        let mut valid_len = 0;

        for (depth, &idx) in self.selected_path.clone().iter().enumerate() {
            if items.is_empty() {
                // Parent folder is now empty — stop here, select the parent
                break;
            }
            let clamped = idx.min(items.len() - 1);
            self.selected_path[depth] = clamped;
            valid_len = depth + 1;

            match &items[clamped].item_type {
                SidebarItemType::Folder(f) if f.open => items = &f.items,
                _ => break,
            }
        }

        self.selected_path.truncate(valid_len);

        // Final safety: if the root list is empty, clear the path entirely
        if self.items.is_empty() {
            self.selected_path.clear();
        }
    }
}

impl Folder for Sidebar {
    fn items(&self) -> &Vec<SidebarItem> {
        &self.items
    }

    fn items_mut(&mut self) -> &mut Vec<SidebarItem> {
        &mut self.items
    }
}


fn collect_visible_paths(items: &[SidebarItem], prefix: &[usize], out: &mut Vec<Vec<usize>>) {
    for (i, item) in items.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(i);
        out.push(path.clone());

        if let SidebarItemType::Folder(ref folder) = item.item_type {
            if folder.open {
                collect_visible_paths(&folder.items, &path, out);
            }
        }
    }
}

fn activate_at(items: &mut [SidebarItem], path: &[usize]) {
    let (&head, tail) = match path.split_first() {
        Some(x) => x,
        None => return,
    };
    if let Some(item) = items.get_mut(head) {
        if tail.is_empty() {
            item.activate();
        } else if let SidebarItemType::Folder(ref mut folder) = item.item_type {
            activate_at(&mut folder.items, tail);
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
            let child_path = if self.selected_path.first() == Some(&i) {
                &self.selected_path[1..]
            } else {
                &[]
            };

            let item_height = item.get_height();
            item.render_at(
                Rect { x: area.x, y, width: area.width, height: item_height },
                buf,
                is_selected,
                child_path,
            );
            y += item_height;
        }
    }
}
