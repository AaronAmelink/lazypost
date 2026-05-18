use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::helpers::text_input::TextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    VarKey(usize),
    VarValue(usize),
}

pub struct EnvironmentEditor {
    pub is_open: bool,
    pub finished: bool,
    var_rows: Vec<(TextInput, TextInput)>,
    focused: Focus,
    editing: bool,
}

impl EnvironmentEditor {
    pub fn new(variables: HashMap<String, String>) -> Self {
        let mut var_rows: Vec<(TextInput, TextInput)> = variables
            .iter()
            .map(|(k, v)| {
                let mut ki = TextInput::new("Key");
                let mut vi = TextInput::new("Value");
                ki.set_value(k.clone());
                vi.set_value(v.clone());
                (ki, vi)
            })
            .collect();
        // Stable display order: sort by key.
        var_rows.sort_by(|a, b| a.0.value().cmp(b.0.value()));
        if var_rows.is_empty() {
            var_rows.push((TextInput::new("Key"), TextInput::new("Value")));
        }
        Self {
            is_open: true,
            finished: false,
            var_rows,
            focused: Focus::VarKey(0),
            editing: false,
        }
    }

    /// Collect the current row state into a HashMap. Rows with empty keys are
    /// dropped. Later rows with duplicate keys overwrite earlier ones.
    pub fn collect(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for (k, v) in &self.var_rows {
            let key = k.value().to_string();
            if !key.is_empty() {
                out.insert(key, v.value().to_string());
            }
        }
        out
    }

    fn disable_all(&mut self) {
        for (k, v) in &mut self.var_rows {
            k.disable();
            v.disable();
        }
    }

    fn enable_focused(&mut self) {
        self.disable_all();
        match self.focused {
            Focus::VarKey(i) => {
                if let Some((k, _)) = self.var_rows.get_mut(i) {
                    k.enable();
                }
            }
            Focus::VarValue(i) => {
                if let Some((_, v)) = self.var_rows.get_mut(i) {
                    v.enable();
                }
            }
        }
    }

    fn focus_list(&self) -> Vec<Focus> {
        let mut out = Vec::with_capacity(self.var_rows.len() * 2);
        for i in 0..self.var_rows.len() {
            out.push(Focus::VarKey(i));
            out.push(Focus::VarValue(i));
        }
        out
    }

    fn move_focus(&mut self, delta: i32) {
        let list = self.focus_list();
        if list.is_empty() {
            return;
        }
        let pos = list.iter().position(|f| *f == self.focused).unwrap_or(0) as i32;
        let new_pos = (pos + delta).rem_euclid(list.len() as i32) as usize;
        self.focused = list[new_pos];
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
                        if let Some((k, _)) = self.var_rows.get_mut(i) {
                            k.handle_event(event);
                        }
                    }
                    Focus::VarValue(i) => {
                        if let Some((_, v)) = self.var_rows.get_mut(i) {
                            v.handle_event(event);
                        }
                    }
                }
            }
            return;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_focus(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_focus(-1),
            KeyCode::Char('e') => {
                self.editing = true;
                self.enable_focused();
            }
            KeyCode::Char('a') => {
                self.var_rows
                    .push((TextInput::new("Key"), TextInput::new("Value")));
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
                Constraint::Length(1), // header
                Constraint::Min(3),    // vars
                Constraint::Length(1), // hint
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new("variables (a: add  d: delete)")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );

        let mut y = 0u16;
        for (i, (key, value)) in self.var_rows.iter().enumerate() {
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
            self.render_input(frame, key_rect, key, self.focused == Focus::VarKey(i));
            self.render_input(frame, val_rect, value, self.focused == Focus::VarValue(i));
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

    fn render_input(&self, frame: &mut Frame, area: Rect, input: &TextInput, is_focused: bool) {
        input.render(frame, area);
        let style = if is_focused && self.editing {
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
}
