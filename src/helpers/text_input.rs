use ratatui::{
    Frame,
    crossterm::event::Event,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph},
};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

/// Single-line text field with an enabled/disabled toggle. When enabled, key
/// events are consumed and the cursor is drawn; when disabled, the field is
/// read-only and the cursor isn't shown.
#[derive(Debug, Default, Clone)]
pub struct TextInput {
    input: Input,
    enabled: bool,
    pub title: String,
}

impl TextInput {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            input: Input::default(),
            enabled: false,
            title: title.into(),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn value(&self) -> &str {
        self.input.value()
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = Input::new(value.into());
    }

    /// Returns `true` if the event was consumed. Only consumes key events
    /// while the input is enabled.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }
        if let Event::Key(_) = event {
            self.input.handle_event(event);
            true
        } else {
            false
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
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(self.title.as_str()),
            );

        frame.render_widget(widget, area);

        if self.enabled {
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1));
        }
    }

    pub fn render_inline(
        &self,
        frame: &mut Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        editing: bool,
    ) {
        let width = area.width.max(1) - 1;
        let scroll = self.input.visual_scroll(width as usize);

        let style = if self.enabled && editing {
            Style::default().fg(Color::Yellow)
        } else if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let widget = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16));

        frame.render_widget(widget, area);

        if self.enabled {
            let x = self.input.visual_cursor().max(scroll) - scroll;
            frame.set_cursor_position((area.x + x as u16, area.y));
        }
    }
}
