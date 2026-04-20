use ratatui::{
    Frame, crossterm::event::{Event, KeyCode, KeyEvent}, style::{Color, Style}, widgets::{Block, BorderType, Paragraph}
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug, Default, Clone)]
pub struct TextInput {
    input: Input,
    enabled: bool,
    title: String,
}

impl TextInput {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            input: Input::default(),
            enabled: false,
            title: title.into(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn clear(&mut self) {
        self.input.reset();
    }

    pub fn value(&self) -> &str {
        self.input.value()
    }

    pub fn value_and_reset(&mut self) -> String {
        self.input.value_and_reset()
    }

    pub fn take_value(&mut self) -> String {
        self.value_and_reset()
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = Input::new(value.into());
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }

        match event {
            Event::Key(key_event) => {

                self.input.handle_event(event);
                true
            }
            _ => false,
        }
    }


    pub fn render(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);

        let style = if self.enabled {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let widget = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().border_type(BorderType::Rounded).title(self.title.as_str()));

        frame.render_widget(widget, area);

        if self.enabled {
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1));
        }
    }
}
