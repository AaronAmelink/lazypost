mod helpers;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::widgets::Block;
use helpers::sidebar::Sidebar;
use helpers::workspace_config::{ConfigFolder, Item, Request, RequestType, WorkspaceConfig};
use std::path::{Path, PathBuf};

const CONFIG_PATH: &str = "workspace.json";

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
    let mut config = WorkspaceConfig::create_from_file(Path::new(CONFIG_PATH))?;

    if config.data.items.is_empty() {
        config.data.items = default_items();
        config.save()?;
    }

    let mut sidebar = Sidebar::new(config.data.items.clone());

    ratatui::run(|terminal| loop {
        terminal.draw(|frame| render(frame, &sidebar))?;
        if handle_events(&mut sidebar)? {
            break Ok(());
        }
    })
}

fn render(frame: &mut ratatui::Frame, sidebar: &Sidebar) {
    let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(frame.area());
    let [top_right, bottom_right] = Layout::vertical([Constraint::Fill(1); 2]).areas(right);

    frame.render_widget(Block::bordered().title("Endpoints"), left);
    frame.render_widget(sidebar.clone(), left.inner(Margin { vertical: 1, horizontal: 1 }));
    frame.render_widget(Block::bordered().title("Request"), top_right);
    frame.render_widget(Block::bordered().title("Response"), bottom_right);
}

fn handle_events(sidebar: &mut Sidebar) -> std::io::Result<bool> {
    let Event::Key(key) = event::read()? else { return Ok(false) };
    if key.kind != KeyEventKind::Press { return Ok(false); }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') => sidebar.select_next(),
        KeyCode::Char('k') => sidebar.select_prev(),
        KeyCode::Enter => sidebar.toggle_selected(),

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
            if sidebar.add_item(sidebar.selected_path.clone(), new_item).is_ok() {
                WorkspaceConfig::save_items_to_file(sidebar.items.clone(), &Path::new(CONFIG_PATH))?;
            }
        }

        KeyCode::Char('d') => {
            if sidebar.remove_selected().is_ok() {
                WorkspaceConfig::save_items_to_file(sidebar.items.clone(), &Path::new(CONFIG_PATH))?;
            }
        }

        _ => {}
    }

    Ok(false)
}
