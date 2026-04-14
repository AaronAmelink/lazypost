use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

#[derive(Clone)]
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

#[derive(Clone)]
pub struct HTTPSidebarItem {
    pub label: RequestType,
}

#[derive(Clone)]
pub struct FolderSidebarItem {
    pub sub_items: Vec<SidebarItem>,
    pub open: bool,
}

impl FolderSidebarItem {
    fn activate(&mut self) {
        self.open = !self.open;
    }

    fn get_height(&self) -> u16 {
        if self.open {
            1 + self.sub_items.iter().map(|i| i.get_height()).sum::<u16>()
        } else {
            1
        }
    }
}

#[derive(Clone)]
pub enum SidebarItemType {
    HTTP(HTTPSidebarItem),
    Folder(FolderSidebarItem),
}

#[derive(Clone)]
pub struct SidebarItem {
    pub name: String,
    pub item_type: SidebarItemType,
}

impl SidebarItem {
    pub fn new_folder(name: String, sub_items: Vec<SidebarItem>) -> Self {
        Self {
            name,
            item_type: SidebarItemType::Folder(FolderSidebarItem {
                sub_items,
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
                    for (i, item) in folder.sub_items.iter().enumerate() {
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

#[derive(Clone)]
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
}

fn collect_visible_paths(items: &[SidebarItem], prefix: &[usize], out: &mut Vec<Vec<usize>>) {
    for (i, item) in items.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(i);
        out.push(path.clone());

        if let SidebarItemType::Folder(ref folder) = item.item_type {
            if folder.open {
                collect_visible_paths(&folder.sub_items, &path, out);
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
            activate_at(&mut folder.sub_items, tail);
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
