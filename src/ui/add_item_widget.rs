use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::model::items::{ConfigFolder, Item, Request, RequestType};
use crate::ui::text_input::TextInput;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Request,
    Folder,
}

pub struct AddItemWidget {
    kind: ItemKind,
    name: TextInput,
    editing: bool,
    pub is_open: bool,
    pub finished_item: Option<Item>,
}

impl AddItemWidget {
    pub fn new() -> Self {
        let mut name = TextInput::new("Name");
        name.enable();
        Self {
            kind: ItemKind::Request,
            name,
            editing: true,
            is_open: true,
            finished_item: None,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn handle_event(&mut self, event: &Event) -> Result<bool, &'static str> {
        let Event::Key(key) = event else {
            return Ok(true);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.name.disable();
                }
                KeyCode::Enter => {
                    self.confirm();
                }
                _ => {
                    self.name.handle_event(event);
                }
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Tab => {
                self.kind = match self.kind {
                    ItemKind::Request => ItemKind::Folder,
                    ItemKind::Folder => ItemKind::Request,
                };
            }
            KeyCode::Char('e') => {
                self.editing = true;
                self.name.enable();
            }
            KeyCode::Enter => {
                self.confirm();
            }
            KeyCode::Esc => {
                self.is_open = false;
            }
            _ => {}
        }
        Ok(true)
    }

    fn confirm(&mut self) {
        let name = self.name.value().trim().to_string();
        let name = if name.is_empty() {
            match self.kind {
                ItemKind::Request => "New Request".to_string(),
                ItemKind::Folder => "New Folder".to_string(),
            }
        } else {
            name
        };
        self.finished_item = Some(match self.kind {
            ItemKind::Request => Item::Request(Request {
                name,
                request_type: RequestType::Get,
                url: String::new(),
                headers: None,
                body: None,
                auth: None,
                params: None,
                url_vars: None,
                capture: None,
            }),
            ItemKind::Folder => Item::Folder(ConfigFolder {
                name,
                items: vec![],
            }),
        });
        self.is_open = false;
    }

    pub fn render_modal(&mut self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(50);
        let modal_height = 9;
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
                .title(" Add Item "),
            modal_area,
        );

        let inner = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // name input
                Constraint::Length(1), // kind selector
                Constraint::Length(1), // hint
            ])
            .split(inner);

        // Name input
        self.name.render(frame, chunks[0]);
        let border_style = if self.editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(border_style)
                .title("Name"),
            chunks[0],
        );

        // Kind selector
        let req_style = if self.kind == ItemKind::Request {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let folder_style = if self.kind == ItemKind::Folder {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let selector = Line::from(vec![
            Span::styled(
                if self.kind == ItemKind::Request {
                    "[Request]"
                } else {
                    " Request "
                },
                req_style,
            ),
            Span::raw("  "),
            Span::styled(
                if self.kind == ItemKind::Folder {
                    "[Folder]"
                } else {
                    " Folder "
                },
                folder_style,
            ),
            Span::raw("   "),
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(selector), chunks[1]);

        // Hint
        let hint = if self.editing {
            "Enter: confirm  Esc: stop editing"
        } else {
            "e: edit  Tab: toggle type  Enter: create  Esc: cancel"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    }
}
