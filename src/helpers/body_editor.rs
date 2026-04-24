use crossterm::event::Event;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType},
    Frame,
};
use ratatui_textarea::TextArea;

pub struct BodyEditor {
    textarea: TextArea<'static>,
    pub enabled: bool,
}

impl Default for BodyEditor {
    fn default() -> Self {
        Self {
            textarea: TextArea::default(),
            enabled: false,
        }
    }
}

impl std::fmt::Debug for BodyEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BodyEditor").field("enabled", &self.enabled).finish()
    }
}

impl Clone for BodyEditor {
    fn clone(&self) -> Self {
        let mut ta = TextArea::from(self.textarea.lines().to_vec());
        ta.set_style(self.textarea.style());
        Self { textarea: ta, enabled: self.enabled }
    }
}

impl BodyEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn enable(&mut self)  { self.enabled = true; }
    pub fn disable(&mut self) { self.enabled = false; }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled { return false; }
        use ratatui_textarea::Input;
        self.textarea.input(Input::from(event.clone()));
        true
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, title: &str) {
        let border_style = if self.enabled {
            Style::default().fg(Color::Yellow)
        } else if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default()
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title.to_owned())
            .border_style(border_style);

        self.textarea.set_block(block);
        self.textarea.set_cursor_style(if self.enabled {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default()
        });

        frame.render_widget(&self.textarea, area);
    }
}
