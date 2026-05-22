use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::net::http_client::{ExecutedResponse, HttpError};
use crate::ui::json_highlight::highlight_json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseTab {
    Body,
    Headers,
}

pub struct ResponseView {
    sub_tab: ResponseTab,
    scroll: u16,
    pretty_cache: Option<String>,
    pub last_error: Option<String>,
}

impl Default for ResponseView {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseView {
    pub fn new() -> Self {
        Self {
            sub_tab: ResponseTab::Body,
            scroll: 0,
            pretty_cache: None,
            last_error: None,
        }
    }

    pub fn set_response(&mut self, resp: Option<&ExecutedResponse>) {
        self.scroll = 0;
        self.last_error = None;
        self.pretty_cache = resp.and_then(|r| {
            let ct = r.content_type.as_deref().unwrap_or("");
            if ct.to_ascii_lowercase().contains("json") {
                serde_json::from_slice::<serde_json::Value>(&r.body)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
            } else {
                None
            }
        });
    }

    pub fn set_error(&mut self, err: &HttpError) {
        self.scroll = 0;
        self.pretty_cache = None;
        self.last_error = Some(err.to_string());
    }

    pub fn handle_event(&mut self, event: &Event) {
        let Event::Key(key) = event else { return };
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            // [ / ] toggle sub-tab (matches editor's sub-tab keys).
            // h/l also toggle since this pane has no horizontal motion.
            KeyCode::Char('[') | KeyCode::Char(']') | KeyCode::Char('h') | KeyCode::Char('l') => {
                self.sub_tab = match self.sub_tab {
                    ResponseTab::Body => ResponseTab::Headers,
                    ResponseTab::Headers => ResponseTab::Body,
                };
                self.scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Char('g') => self.scroll = 0,
            _ => {}
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        resp: Option<&ExecutedResponse>,
        in_flight: bool,
        spinner: u8,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status line
                Constraint::Length(1), // sub-tab row
                Constraint::Min(1),    // body
            ])
            .split(area);

        // Status line
        let status_line = if in_flight {
            let glyphs = ['|', '/', '-', '\\'];
            let g = glyphs[(spinner as usize) % glyphs.len()];
            Line::from(vec![Span::styled(
                format!(" {g} sending..."),
                Style::default().fg(Color::Yellow),
            )])
        } else if let Some(err) = &self.last_error {
            Line::from(vec![Span::styled(
                format!(" ✗ {err}"),
                Style::default().fg(Color::Red),
            )])
        } else if let Some(r) = resp {
            let color = status_color(r.status);
            Line::from(vec![
                Span::styled(
                    format!(" {} {} ", r.status, r.status_text),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}ms  {} bytes", r.elapsed_ms, r.body.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            Line::from(Span::styled(
                " press 's' on sidebar to send the selected request",
                Style::default().fg(Color::DarkGray),
            ))
        };
        frame.render_widget(Paragraph::new(status_line), chunks[0]);

        // Sub-tab row
        let body_label = if self.sub_tab == ResponseTab::Body {
            " [Body] "
        } else {
            "  Body  "
        };
        let hdr_label = if self.sub_tab == ResponseTab::Headers {
            " [Headers] "
        } else {
            "  Headers  "
        };
        let body_style = if self.sub_tab == ResponseTab::Body {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let hdr_style = if self.sub_tab == ResponseTab::Headers {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let tab_line = Line::from(vec![
            Span::styled(body_label, body_style),
            Span::raw("  "),
            Span::styled(hdr_label, hdr_style),
            Span::raw("   "),
            Span::styled(
                "[/] or h/l toggle  j/k scroll",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(tab_line), chunks[1]);

        // Body area
        let body_area = chunks[2];
        let block = Block::bordered().border_type(BorderType::Rounded);
        let inner = block.inner(body_area);
        frame.render_widget(block, body_area);

        let body_text: Text = match (resp, self.sub_tab) {
            (None, _) => Text::default(),
            (Some(r), ResponseTab::Body) => {
                if let Some(json) = &self.pretty_cache {
                    highlight_json(json)
                } else {
                    Text::raw(String::from_utf8_lossy(&r.body).to_string())
                }
            }
            (Some(r), ResponseTab::Headers) => Text::raw(
                r.headers
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        };

        frame.render_widget(
            Paragraph::new(body_text)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .alignment(Alignment::Left),
            inner,
        );
    }
}

fn status_color(status: u16) -> Color {
    match status {
        100..=199 => Color::Cyan,
        200..=299 => Color::Green,
        300..=399 => Color::Cyan,
        400..=499 => Color::Yellow,
        500..=599 => Color::Red,
        _ => Color::White,
    }
}
