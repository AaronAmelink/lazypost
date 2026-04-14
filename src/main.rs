mod helpers;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Block,
};
use helpers::sidebar::{Sidebar, SidebarItem, RequestType};

fn main() -> std::io::Result<()> {
    let mut sidebar = Sidebar::new(vec![
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
    ]);

    ratatui::run(|terminal| -> Result<(), std::io::Error> {
        loop {
            terminal.draw(|frame| render(frame, &sidebar))?;
            if handle_events(&mut sidebar)? {
                break Ok(());
            }
        }
    })
}

fn render(frame: &mut ratatui::Frame, sidebar: &Sidebar) {
    let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(frame.area());
    let [top_right, bottom_right] = Layout::vertical([Constraint::Fill(1); 2]).areas(right);

    // Render sidebar block border
    frame.render_widget(Block::bordered().title("Endpoints"), left);

    // Render sidebar items inside the block (accounting for border)
    let sidebar_area = left.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(sidebar.clone(), sidebar_area);

    frame.render_widget(Block::bordered().title("Responses"), bottom_right);
    frame.render_widget(Block::bordered().title("Request"), top_right);
}

fn handle_events(sidebar: &mut Sidebar) -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('j') => sidebar.next(),
            KeyCode::Char('k') => sidebar.previous(),
            KeyCode::Enter => sidebar.activate(),
            _ => {}
        },
        _ => {}
    }
    Ok(false)
}
