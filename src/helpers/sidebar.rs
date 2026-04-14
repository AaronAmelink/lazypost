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

trait SidebarItemBehaviour {
    fn render(&self, name: &str, area: Rect, buf: &mut Buffer, is_selected: bool);
    fn activate(&mut self);
    fn get_height(&self) -> u16;
}

impl SidebarItemBehaviour for HTTPSidebarItem {
    fn render(&self, name: &str, area: Rect, buf: &mut Buffer, is_selected: bool) {
        let method_str = self.label.as_str();
        let method_color = self.label.color();

        let prefix = if is_selected { "> " } else { "  " };
        let line_str = format!("{}{} ", prefix, name);
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

        // Highlight the method in its specific color
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

    fn activate(&mut self) {
    }

    fn get_height(&self) -> u16 {
        1
    }
}

#[derive(Clone)]
pub struct FolderSidebarItem {
    pub sub_items: Vec<SidebarItem>,
    pub open: bool,
    pub selected_item: usize,
}

impl SidebarItemBehaviour for FolderSidebarItem {
    fn render(&self, name: &str, area: Rect, buf: &mut Buffer, is_selected: bool) {
        let prefix;
        if self.open {
            prefix = "v ";
        } else if is_selected {
            prefix = "> ";
        } else {
            prefix = "  ";
        }
        let line_str = format!("{}/{}", prefix, name);

        let style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        buf.set_string(area.x, area.y, line_str, style);

        if self.open {
            let mut y = area.y + 1;
            for item in &self.sub_items {
                item.render(Rect { x: area.x, y, width: area.width, height: 1 }, buf, false);
                y += item.get_height();
            }
        }
    }

    fn activate(&mut self) {
        self.open = !self.open;
    }

    fn get_height(&self) -> u16 {
        if self.open {
            self.sub_items.len() as u16 + 1
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

impl SidebarItemBehaviour for SidebarItemType {
    fn render(&self, name: &str, area: Rect, buf: &mut Buffer, is_selected: bool) {
        match self {
            SidebarItemType::HTTP(http) => http.render(name, area, buf, is_selected),
            SidebarItemType::Folder(folder) => folder.render(name, area, buf, is_selected),
        }
    }

    fn activate(&mut self) {
        match self {
            SidebarItemType::HTTP(http) => http.activate(),
            SidebarItemType::Folder(folder) => folder.activate(),
        }
    }

    fn get_height(&self) -> u16 {
        match self {
            SidebarItemType::HTTP(http) => http.get_height(),
            SidebarItemType::Folder(folder) => folder.get_height(),
        }
    }
}

#[derive(Clone)]
pub struct SidebarItem {
    pub name: String,
    pub item_type: SidebarItemType,
}

impl SidebarItem {
    pub fn new_folder(name: String) -> Self {
        Self {
            name,
            item_type: SidebarItemType::Folder(FolderSidebarItem
                {
                    sub_items: vec![],
                    open: false
                }),
        }
    }

    pub fn new_http(name: String, label: RequestType) -> Self {
        Self {
            name,
            item_type: SidebarItemType::HTTP(HTTPSidebarItem { label }),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, is_selected: bool) {
        self.item_type.render(&self.name, area, buf, is_selected);
    }

    pub fn activate(&mut self) {
        self.item_type.activate();
    }

    pub fn get_height(&self) -> u16 {
        self.item_type.get_height()
    }
}

#[derive(Clone)]
pub struct Sidebar {
    pub selected_item: usize,
    pub items: Vec<SidebarItem>,
}

impl Sidebar {
    pub fn new(items: Vec<SidebarItem>) -> Self {
        Self {
            selected_item: 0,
            items,
        }
    }

    pub fn next(&mut self) {
        if self.selected_item < self.items.len().saturating_sub(1) {
            self.selected_item += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_item > 0 {
            self.selected_item -= 1;
        }
    }

    pub fn activate(&mut self) {
        self.items[self.selected_item].activate();
    }
}

impl Widget for Sidebar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        let mut y = area.y;

        for (index, item) in self.items.iter().enumerate() {
            if y >= area.y + area.height {
                break;
            }

            let is_selected = index == self.selected_item;

            item.render(Rect { x: area.x, y, width: area.width, height: 1 }, buf, is_selected);
            y += item.get_height();
        }
    }
}
