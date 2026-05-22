use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType};

use crate::ui::text_input::TextInput;

/// A reusable key-value row used by params, headers, url vars, body form rows,
/// and the environment editor. The `enabled` flag is used by params to toggle
/// whether a row is sent.
#[derive(Debug, Clone)]
pub struct KvRow {
    pub key: TextInput,
    pub value: TextInput,
    pub enabled: bool,
}

impl KvRow {
    pub fn new(key_title: &str, val_title: &str) -> Self {
        Self {
            key: TextInput::new(key_title.to_owned()),
            value: TextInput::new(val_title.to_owned()),
            enabled: true,
        }
    }

    pub fn from_pair(k: &str, v: &str, enabled: bool) -> Self {
        let mut row = Self::new("Key", "Value");
        row.key.set_value(k);
        row.value.set_value(v);
        row.enabled = enabled;
        row
    }
}

/// Renders a text input with a focused border overlay. The border color is
/// yellow when editing, cyan when merely focused, and invisible otherwise.
pub fn render_input(
    frame: &mut Frame,
    area: Rect,
    input: &TextInput,
    is_focused: bool,
    editing: bool,
) {
    input.render(frame, area);
    let style = if is_focused && editing {
        Style::default().fg(Color::Yellow)
    } else if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        return;
    };
    frame.render_widget(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(style)
            .title(input.title.clone()),
        area,
    );
}
