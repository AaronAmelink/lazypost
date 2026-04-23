mod helpers;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::widgets::Block;
use helpers::sidebar::Sidebar;
use helpers::items::{Item, RequestType, Request, ConfigFolder};
use helpers::workspace_config::{WorkspaceConfig};
use std::path::Path;

use crate::helpers::add_item_widget::AddItemWidget;

const CONFIG_PATH: &str = "workspace.json";

struct App {
    sidebar: Sidebar,
    add_item_widget: Option<AddItemWidget>,
    config: WorkspaceConfig,
}

impl App {
    fn new() -> Self {
        Self {
            sidebar: Sidebar::new(default_items()),
            add_item_widget: None,
            config: WorkspaceConfig::new_empty(),
        }
    }

    fn handle_events(&mut self, event: &Event) -> Result<bool, &str> {
        let Event::Key(key) = event else { return Err("Error") };
        if key.kind != KeyEventKind::Press { return Err("Error") }

        if let Some(widget) = &mut self.add_item_widget {
            widget.handle_event(event);
            if !widget.is_open {
                self.add_item_widget = None;
            }
        } else {
            self.sidebar.handle_event(event)?;
        }

        match key.code {
            KeyCode::Char('N') => {
                self.add_item_widget = Some(AddItemWidget::new());
            },
            KeyCode::Char('q') => {
                return Ok(false);
            },
            _ => {}
        }
        return Ok(true);
    }
}

fn default_items() -> Vec<Item> {
    let req = |name: &str, request_type: RequestType| Item::Request(Request {
        name: name.into(),
        request_type,
        url: String::new(),
        headers: None,
        body: None,
        auth: None,
        params: None,
    });

    vec![
        req("Get Users", RequestType::Get),
        req("Create User", RequestType::Post),
        req("Update User", RequestType::Put),
        req("Delete User", RequestType::Delete),
        req("Get Posts", RequestType::Get),
        Item::Folder(ConfigFolder {
            name: "User Actions".into(),
            items: vec![
                Item::Folder(ConfigFolder {
                    name: "Nested Folder".into(),
                    items: vec![req("Delete User", RequestType::Delete)],
                }),
                Item::Folder(ConfigFolder {
                    name: "Another Nested".into(),
                    items: vec![req("Delete User", RequestType::Delete)],
                }),
            ],
        }),
    ]
}

fn main() {
    let mut app = App::new();
    app.config = WorkspaceConfig::create_from_file(Path::new(CONFIG_PATH)).unwrap_or_else(|_| WorkspaceConfig::new_empty());

    if app.config.data.items.is_empty() {
        app.config.data.items = default_items();
        let _ = app.config.save();
    }
    ratatui::run(|terminal| loop {
        let _ = terminal.draw(|frame| render(frame, &mut app));
        if !handle_events(&mut app).unwrap_or(false) {
            break;
        }
    });
}



fn render(frame: &mut ratatui::Frame, app: &mut App) {
    let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(frame.area());
    let [top_right, bottom_right] = Layout::vertical([Constraint::Fill(1); 2]).areas(right);

    frame.render_widget(Block::bordered().title("Endpoints"), left);
    frame.render_widget(app.sidebar.clone(), left.inner(Margin { vertical: 1, horizontal: 1 }));
    frame.render_widget(Block::bordered().title("Request"), top_right);
    frame.render_widget(Block::bordered().title("Response"), bottom_right);

    if let Some(widget) = &mut app.add_item_widget {
        widget.render_modal(frame, frame.area());
    }
}

fn handle_events(app: &mut App) -> Result<bool, &str> {
    let event = event::read().map_err(|_| "Error reading event")?;
    app.handle_events(&event)
}
