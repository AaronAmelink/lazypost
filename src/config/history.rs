use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::items::Request;
use crate::net::http_client::ExecutedResponse;

const DEFAULT_CAP: usize = 500;
const INLINE_BODY_CAP: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSummary {
    pub status: u16,
    pub status_text: String,
    pub elapsed_ms: u64,
    pub content_type: Option<String>,
    pub byte_size: usize,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

impl ResponseSummary {
    fn from_response(resp: &ExecutedResponse) -> Self {
        let body = if resp.body.len() > INLINE_BODY_CAP {
            resp.body[..INLINE_BODY_CAP].to_vec()
        } else {
            resp.body.clone()
        };
        Self {
            status: resp.status,
            status_text: resp.status_text.clone(),
            elapsed_ms: resp.elapsed_ms,
            content_type: resp.content_type.clone(),
            byte_size: resp.body.len(),
            body,
            headers: resp.headers.clone(),
        }
    }

    pub fn to_response(&self) -> ExecutedResponse {
        ExecutedResponse {
            status: self.status,
            status_text: self.status_text.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            content_type: self.content_type.clone(),
            elapsed_ms: self.elapsed_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub request_snapshot: Request,
    pub response: ResponseSummary,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryFile {
    entries: VecDeque<HistoryEntry>,
}

#[derive(Debug)]
pub struct History {
    pub entries: VecDeque<HistoryEntry>,
    pub cap: usize,
    pub path: PathBuf,
    pub selected: usize,
}

impl History {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("history.json");
        let entries = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<HistoryFile>(&s).ok())
            .map(|h| h.entries)
            .unwrap_or_default();
        Self {
            entries,
            cap: DEFAULT_CAP,
            path,
            selected: 0,
        }
    }

    pub fn push(&mut self, request: Request, response: &ExecutedResponse) {
        let entry = HistoryEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            request_snapshot: request,
            response: ResponseSummary::from_response(response),
        };
        self.entries.push_front(entry);
        while self.entries.len() > self.cap {
            self.entries.pop_back();
        }
        self.selected = 0;
        let _ = self.save();
    }

    pub fn save(&self) -> std::io::Result<()> {
        let file = HistoryFile {
            entries: self.entries.clone(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
        std::fs::write(&self.path, json)
    }

    pub fn selected_entry(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.selected)
    }

    pub fn handle_event(&mut self, event: &Event) -> HistoryAction {
        let Event::Key(key) = event else {
            return HistoryAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return HistoryAction::None;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.entries.is_empty() && self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
                HistoryAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                HistoryAction::None
            }
            KeyCode::Char('d') => {
                if self.selected < self.entries.len() {
                    self.entries.remove(self.selected);
                    if self.selected >= self.entries.len() && self.selected > 0 {
                        self.selected -= 1;
                    }
                    let _ = self.save();
                }
                HistoryAction::None
            }
            KeyCode::Char('D') => {
                self.entries.clear();
                self.selected = 0;
                let _ = self.save();
                HistoryAction::None
            }
            KeyCode::Enter => {
                if self.entries.get(self.selected).is_some() {
                    HistoryAction::Restore
                } else {
                    HistoryAction::None
                }
            }
            KeyCode::Esc | KeyCode::Char('H') | KeyCode::Char('q') => HistoryAction::Close,
            _ => HistoryAction::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(90);
        let modal_height = screen_area.height.min(30);
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
                .title(" History "),
            modal_area,
        );
        let inner = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        if self.entries.is_empty() {
            frame.render_widget(
                Paragraph::new("(no history yet — send a request)")
                    .style(Style::default().fg(Color::DarkGray))
                    .alignment(Alignment::Center),
                chunks[0],
            );
        } else {
            let visible = chunks[0].height as usize;
            let start = if self.selected >= visible {
                self.selected - visible + 1
            } else {
                0
            };
            let lines: Vec<Line> = self
                .entries
                .iter()
                .enumerate()
                .skip(start)
                .take(visible)
                .map(|(i, e)| {
                    let ts = e.timestamp.format("%H:%M:%S").to_string();
                    let status_color = match e.response.status {
                        200..=299 => Color::Green,
                        300..=399 => Color::Cyan,
                        400..=499 => Color::Yellow,
                        500..=599 => Color::Red,
                        _ => Color::White,
                    };
                    let url = e.request_snapshot.url.clone();
                    let marker = if i == self.selected { "▶ " } else { "  " };
                    let row_style = if i == self.selected {
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    Line::from(vec![
                        Span::styled(marker, row_style),
                        Span::styled(format!("[{ts}] "), row_style.fg(Color::DarkGray)),
                        Span::styled(
                            format!("{:3} ", e.response.status),
                            row_style.fg(status_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:6} ", e.request_snapshot.request_type.as_str()),
                            row_style.fg(e.request_snapshot.request_type.color()),
                        ),
                        Span::styled(
                            format!("{:>5}ms ", e.response.elapsed_ms),
                            row_style.fg(Color::DarkGray),
                        ),
                        Span::styled(url, row_style),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), chunks[0]);
        }

        frame.render_widget(
            Paragraph::new("j/k navigate  Enter restore  d delete  D clear  Esc/H close")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            chunks[1],
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HistoryAction {
    None,
    Close,
    Restore,
}
