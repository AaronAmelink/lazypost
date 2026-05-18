use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::{Block, BorderType, Clear};

use crate::helpers::items::Item;
use crate::helpers::request_editor::RequestEditor;

pub struct AddItemWidget {
    pub editor: RequestEditor,
    pub is_open: bool,
    /// Set to Some(item) when the user presses Enter to confirm.
    pub finished_item: Option<Item>,
}

impl AddItemWidget {
    pub fn new() -> Self {
        Self {
            editor: RequestEditor::new(),
            is_open: true,
            finished_item: None,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Result<bool, &'static str> {
        let Event::Key(key) = event else {
            return Ok(true);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        if self.editor.is_editing() {
            // While editing a field, forward everything (including Esc) to the editor.
            self.editor.handle_event(event);
            return Ok(true);
        }

        match key.code {
            KeyCode::Enter => match self.editor.to_request() {
                Ok(req) => {
                    self.finished_item = Some(Item::Request(req));
                    self.is_open = false;
                }
                Err(_) => {
                    // validation_error is now set on the editor; let the next render show it.
                }
            },
            KeyCode::Esc => {
                self.is_open = false;
            }
            _ => {
                self.editor.handle_event(event);
            }
        }
        Ok(true)
    }

    pub fn render_modal(&mut self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(72);
        let modal_height = screen_area.height.min(32);
        let x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
        let y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect {
            x,
            y,
            width: modal_width,
            height: modal_height,
        };
        frame.render_widget(Clear, modal_area);
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Add Request "),
            modal_area,
        );

        let inner = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        self.editor.render(frame, inner, true);
    }
}
