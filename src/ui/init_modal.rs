use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

pub enum InitAction {
    None,
    Confirm,
    Cancel,
}

pub struct InitModal {
    pub cwd: String,
}

impl InitModal {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        Self { cwd }
    }

    pub fn handle_event(&self, event: &Event) -> InitAction {
        let Event::Key(key) = event else {
            return InitAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return InitAction::None;
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => InitAction::Confirm,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => InitAction::Cancel,
            _ => InitAction::None,
        }
    }

    pub fn render_modal(&self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(60);
        let modal_height = 9;
        let x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
        let y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect { x, y, width: modal_width, height: modal_height };
        frame.render_widget(Clear, modal_area);
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Initialize Workspace "),
            modal_area,
        );

        let inner = modal_area.inner(Margin { vertical: 1, horizontal: 2 });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("No workspace file found in:").style(Style::default().fg(Color::White)),
            chunks[0],
        );

        let cwd_display = if self.cwd.len() as u16 > modal_width.saturating_sub(4) {
            format!("…{}", &self.cwd[self.cwd.len().saturating_sub((modal_width as usize).saturating_sub(5))..])
        } else {
            self.cwd.clone()
        };
        frame.render_widget(
            Paragraph::new(cwd_display).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new("Initialize lazypost workspace here?").style(Style::default().fg(Color::White)),
            chunks[2],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("[y] Yes", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("   "),
                Span::styled("[n] No / quit", Style::default().fg(Color::Red)),
            ])),
            chunks[4],
        );
    }
}
