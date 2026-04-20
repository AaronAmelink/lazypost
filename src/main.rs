mod helpers;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::widgets::Block;
use helpers::sidebar::Sidebar;
use helpers::items::{Item, RequestType, Request, ConfigFolder};
use helpers::workspace_config::{WorkspaceConfig};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
        }

        match key.code {
            KeyCode::Char('N') => {
                self.add_item_widget = Some(AddItemWidget::new());
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

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.config = WorkspaceConfig::create_from_file(Path::new(CONFIG_PATH))?;

    if app.config.data.items.is_empty() {
        app.config.data.items = default_items();
        app.config.save()?;
    }
    ratatui::run(|terminal| loop {
        terminal.draw(|frame| render(frame, &mut app))?;
        if handle_events(&mut app)? {
            break Ok(());
        }
    })
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

fn handle_events(app: &mut App) -> std::io::Result<bool> {
    let event = event::read()?;
    let Event::Key(key) = event else { return Ok(false) };
    if key.kind != KeyEventKind::Press { return Ok(false); }

    if app.handle_events(&event).is_err() {
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') => app.sidebar.select_next(),
        KeyCode::Char('k') => app.sidebar.select_prev(),
        KeyCode::Enter => app.sidebar.toggle_selected(),

        KeyCode::Char('n') => {
            let new_item = Item::Request(Request {
                name: "New Request".into(),
                request_type: RequestType::Get,
                url: String::new(),
                headers: None,
                body: None,
                auth: None,
                params: None,
            });
            if app.sidebar.add_item(app.sidebar.selected_path.clone(), new_item).is_ok() {
                WorkspaceConfig::save_items_to_file(app.sidebar.items.clone(), &Path::new(CONFIG_PATH))?;
            }
        }

        KeyCode::Char('d') => {
            if app.sidebar.remove_selected().is_ok() {
                WorkspaceConfig::save_items_to_file(app.sidebar.items.clone(), &Path::new(CONFIG_PATH))?;
            }
        }
        _ => {}
    }

    Ok(false)
}
