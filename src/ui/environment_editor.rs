use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::ui::kv_row::{KvRow, render_input};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    VarKey(usize),
    VarValue(usize),
}

pub struct EnvironmentEditor {
    pub is_open: bool,
    pub finished: bool,
    var_rows: Vec<KvRow>,
    focused: Focus,
    editing: bool,
}

impl EnvironmentEditor {
    pub fn new(variables: HashMap<String, String>) -> Self {
        let mut var_rows: Vec<KvRow> = variables
            .iter()
            .map(|(k, v)| KvRow::from_pair(k, v, true))
            .collect();
        var_rows.sort_by(|a, b| a.key.value().cmp(b.key.value()));
        if var_rows.is_empty() {
            var_rows.push(KvRow::new("Key", "Value"));
        }
        Self {
            is_open: true,
            finished: false,
            var_rows,
            focused: Focus::VarKey(0),
            editing: false,
        }
    }

    pub fn collect(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for row in &self.var_rows {
            let key = row.key.value().to_string();
            if !key.is_empty() {
                out.insert(key, row.value.value().to_string());
            }
        }
        out
    }

    fn disable_all(&mut self) {
        for row in &mut self.var_rows {
            row.key.disable();
            row.value.disable();
        }
    }

    fn enable_focused(&mut self) {
        self.disable_all();
        match self.focused {
            Focus::VarKey(i) => {
                if let Some(row) = self.var_rows.get_mut(i) {
                    row.key.enable();
                }
            }
            Focus::VarValue(i) => {
                if let Some(row) = self.var_rows.get_mut(i) {
                    row.value.enable();
                }
            }
        }
    }

    fn row_index(&self) -> usize {
        match self.focused {
            Focus::VarKey(i) | Focus::VarValue(i) => i,
        }
    }

    fn move_vertical(&mut self, delta: i32) {
        let len = self.var_rows.len();
        if len == 0 {
            return;
        }
        let i = self.row_index() as i32 + delta;
        if i < 0 || i >= len as i32 {
            return;
        }
        self.focused = match self.focused {
            Focus::VarKey(_) => Focus::VarKey(i as usize),
            Focus::VarValue(_) => Focus::VarValue(i as usize),
        };
    }

    pub fn handle_event(&mut self, event: &Event) {
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.editing {
            if key.code == KeyCode::Esc {
                self.editing = false;
                self.disable_all();
            } else {
                match self.focused {
                    Focus::VarKey(i) => {
                        if let Some(row) = self.var_rows.get_mut(i) {
                            row.key.handle_event(event);
                        }
                    }
                    Focus::VarValue(i) => {
                        if let Some(row) = self.var_rows.get_mut(i) {
                            row.value.handle_event(event);
                        }
                    }
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_vertical(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_vertical(-1),
            KeyCode::Char('h') | KeyCode::Left => {
                let i = self.row_index();
                self.focused = Focus::VarKey(i);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let i = self.row_index();
                self.focused = Focus::VarValue(i);
            }
            KeyCode::Char('e') => {
                self.editing = true;
                self.enable_focused();
            }
            KeyCode::Char('a') => {
                self.var_rows.push(KvRow::new("Key", "Value"));
                self.focused = Focus::VarKey(self.var_rows.len() - 1);
            }
            KeyCode::Char('d') => {
                let (Focus::VarKey(i) | Focus::VarValue(i)) = self.focused;
                if i < self.var_rows.len() && self.var_rows.len() > 1 {
                    self.var_rows.remove(i);
                    self.focused = Focus::VarKey(i.min(self.var_rows.len() - 1));
                }
            }
            KeyCode::Enter => {
                self.finished = true;
                self.is_open = false;
            }
            KeyCode::Esc => {
                self.is_open = false;
            }
            _ => {}
        }
    }

    pub fn render_modal(&self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(70);
        let modal_height = screen_area.height.min(28);
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
                .title(" Environment variables "),
            modal_area,
        );

        let inner = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("variables (a: add  d: delete)")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );

        let mut y = 0u16;
        for (i, row) in self.var_rows.iter().enumerate() {
            if y + 3 > chunks[1].height {
                break;
            }
            let half = chunks[1].width / 2;
            let key_rect = Rect {
                x: chunks[1].x,
                y: chunks[1].y + y,
                width: half,
                height: 3,
            };
            let val_rect = Rect {
                x: chunks[1].x + half,
                y: chunks[1].y + y,
                width: chunks[1].width - half,
                height: 3,
            };
            render_input(
                frame,
                key_rect,
                &row.key,
                self.focused == Focus::VarKey(i),
                self.editing,
            );
            render_input(
                frame,
                val_rect,
                &row.value,
                self.focused == Focus::VarValue(i),
                self.editing,
            );
            y += 3;
        }

        let hint = if self.editing {
            "Esc: stop editing"
        } else {
            "j/k navigate  e edit  a/d add/delete  Enter save  Esc cancel"
        };
        frame.render_widget(
            Paragraph::new(hint)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[2],
        );
    }
}
