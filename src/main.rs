mod helpers;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Block,
};
use helpers::sidebar::{Sidebar, SidebarItem, RequestType, Folder};
use helpers::workspace_config::WorkspaceConfig;
use std::path::Path;

const CONFIG_PATH: &str = "workspace.json";

fn main() -> std::io::Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    let mut config = WorkspaceConfig::load_or_create(config_path)?;

    let initial_items = if config.data.items.is_empty() {
        let defaults = vec![
            SidebarItem::new_http("Get Users".to_string(), RequestType::Get),
            SidebarItem::new_http("Create User".to_string(), RequestType::Post),
            SidebarItem::new_http("Update User".to_string(), RequestType::Put),
            SidebarItem::new_http("Delete User".to_string(), RequestType::Delete),
            SidebarItem::new_http("Get Posts".to_string(), RequestType::Get),
            SidebarItem::new_folder("name".to_string(), vec![
                SidebarItem::new_folder("Nested Folder".to_string(), vec![
                    SidebarItem::new_http("Delete User".to_string(), RequestType::Delete),
                ]),
                SidebarItem::new_folder("Nested Folder".to_string(), vec![
                    SidebarItem::new_http("Delete User".to_string(), RequestType::Delete),
                ]),
            ])
        ];
        config.sync_from_sidebar(&defaults)?;
        defaults
    } else {
        config.to_sidebar_items()
    };

    let mut sidebar = Sidebar::new(initial_items);

    ratatui::run(|terminal| -> Result<(), std::io::Error> {
        loop {
            let sidebar_clone = sidebar.clone();
            terminal.draw(|frame| render(frame, &sidebar_clone))?;
            if handle_events(&mut sidebar, &mut config)? {
                break Ok(());
            }
        }
    })
}

fn render(frame: &mut ratatui::Frame, sidebar: &Sidebar) {
    let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(frame.area());
    let [top_right, bottom_right] = Layout::vertical([Constraint::Fill(1); 2]).areas(right);

    frame.render_widget(Block::bordered().title("Endpoints"), left);

    let sidebar_area = left.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(sidebar.clone(), sidebar_area);

    frame.render_widget(Block::bordered().title("Responses"), bottom_right);
    frame.render_widget(Block::bordered().title("Request"), top_right);
}

fn handle_events(sidebar: &mut Sidebar, config: &mut WorkspaceConfig) -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('j') => {
                sidebar.next();
            }
            KeyCode::Char('k') => {
                sidebar.previous();
            }
            KeyCode::Enter => {
                sidebar.activate();
            }
            KeyCode::Char('n') => {
                if sidebar.add_item(
                    sidebar.selected_path.clone(),
                    SidebarItem::new_http("name".to_string(), RequestType::Get),
                ).is_ok() {
                    let _ = config.sync_from_sidebar(&sidebar.items);
                }
            }
            KeyCode::Char('d') => {
                if sidebar.remove_selected().is_ok() {
                    let _ = config.sync_from_sidebar(&sidebar.items);
                }
            }

            _ => {}
        },
        _ => {}
    }
    Ok(false)
}
