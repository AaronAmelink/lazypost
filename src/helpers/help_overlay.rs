use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};

pub struct HelpOverlay {
    pub open: bool,
    scroll: u16,
}

impl Default for HelpOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self {
            open: false,
            scroll: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.scroll = 0;
        }
    }

    /// Returns true if the event was consumed (i.e. overlay handled it).
    pub fn handle_event(&mut self, event: &Event) -> bool {
        let Event::Key(key) = event else { return false };
        if key.kind != KeyEventKind::Press {
            return false;
        }
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.open = false;
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(10);
                true
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                true
            }
            KeyCode::Char('g') => {
                self.scroll = 0;
                true
            }
            _ => true, // swallow other keys while open
        }
    }

    pub fn render(&self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(82);
        let modal_height = screen_area.height.min(38);
        let x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
        let y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;
        let area = Rect {
            x,
            y,
            width: modal_width,
            height: modal_height,
        };

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Help — ? to close "),
            area,
        );
        let inner = area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let mut lines: Vec<Line> = Vec::new();
        section(&mut lines, "Panes");
        kv(&mut lines, "Tab / Shift-Tab", "Cycle panes");
        kv(
            &mut lines,
            "1 / 2 / 3",
            "Jump to Sidebar / Editor / Response",
        );
        blank(&mut lines);

        section(&mut lines, "Global");
        kv(&mut lines, "n", "New request (modal)");
        kv(&mut lines, "s", "Send the currently selected request");
        kv(&mut lines, "w  or  S", "Save the editor's current request");
        kv(&mut lines, "E", "Open environment variables editor");
        kv(&mut lines, "H", "Open request history");
        kv(&mut lines, "?", "Toggle this help");
        kv(&mut lines, "q", "Quit");
        blank(&mut lines);

        section(&mut lines, "Sidebar (pane 1)");
        kv(&mut lines, "j / k", "Move selection up / down");
        kv(&mut lines, "Enter", "Toggle folder open/closed");
        kv(&mut lines, "d", "Delete the selected request or folder");
        blank(&mut lines);

        section(&mut lines, "Request editor (pane 2)");
        kv(
            &mut lines,
            "[ / ]",
            "Cycle sub-tabs (Info / Auth / Body / Params / URL Vars / Headers / Capture)",
        );
        kv(
            &mut lines,
            "j / k",
            "Move between fields in the current sub-tab",
        );
        kv(&mut lines, "e", "Edit the focused field; press Esc to stop");
        kv(
            &mut lines,
            "h / l",
            "Move between columns or cycle selector values",
        );
        kv(
            &mut lines,
            "a",
            "Add a row (Params / URL Vars / Headers / Form / Multipart)",
        );
        kv(&mut lines, "d", "Delete the focused row");
        kv(
            &mut lines,
            "t",
            "Toggle: param/url-var enabled, or multipart text/file",
        );
        kv(
            &mut lines,
            "w  or  Ctrl-leave",
            "Save (auto-saves on pane/selection change)",
        );
        blank(&mut lines);

        section(&mut lines, "Response (pane 3)");
        kv(&mut lines, "[ / ] or h / l", "Toggle Body / Headers");
        kv(&mut lines, "j / k / PageUp / PageDown", "Scroll");
        kv(&mut lines, "g", "Scroll to top");
        blank(&mut lines);

        section(&mut lines, "Environment variables editor (E)");
        kv(&mut lines, "j / k", "Navigate fields");
        kv(&mut lines, "e", "Edit focused field");
        kv(&mut lines, "a / d", "Add / delete a variable row");
        kv(&mut lines, "Enter", "Save and close");
        kv(&mut lines, "Esc", "Cancel without saving");
        blank(&mut lines);

        section(&mut lines, "History (H)");
        kv(&mut lines, "j / k", "Navigate entries");
        kv(
            &mut lines,
            "Enter",
            "Restore request + response into the editor",
        );
        kv(&mut lines, "d", "Delete the highlighted entry");
        kv(&mut lines, "D (shift)", "Clear history");
        blank(&mut lines);

        section(&mut lines, "Environment variables");
        para(
            &mut lines,
            "Anywhere in url / headers / params / body / auth fields, write {{var_name}} to \
             substitute a variable. Missing or empty vars are errors. Open the editor \
             with E to define variables.",
        );
        para(
            &mut lines,
            "Example: with base_url=https://api.example.com and token=abc, a request to \
             {{base_url}}/users with header Authorization: Bearer {{token}} sends to \
             https://api.example.com/users with the real token.",
        );
        blank(&mut lines);

        section(&mut lines, "URL variables");
        para(
            &mut lines,
            "On the URL Vars tab, define key/value pairs and reference them in the URL as \
             <key>. Whitespace inside the brackets is allowed (< key >). Values are URL-encoded. \
             Missing or empty URL vars are errors.",
        );
        blank(&mut lines);

        section(&mut lines, "Capture (predicted response)");
        para(
            &mut lines,
            "Each request has a Capture tab. Put a JSON template using %name% placeholders. \
             After the request runs, the template is walked in parallel with the actual response; \
             %name% slots capture the matching value into the env vars list (visible via E).",
        );
        para(
            &mut lines,
            "Example template:  {\"item\": {\"id\": \"%item_id%\", \"slug\": \"%slug%\"}}",
        );
        para(
            &mut lines,
            "If the response is  {\"item\": {\"id\": 42, \"slug\": \"hello\"}}, the vars get \
             item_id=42 and slug=hello. Subsequent requests can reference {{item_id}}.",
        );
        para(
            &mut lines,
            "Placeholders may also be embedded in a literal string, e.g. \"Bearer %key%\" \
             captures the token portion of \"Bearer abc123\". Multiple placeholders per string \
             are supported (\"%user%:%pass%\").",
        );
        para(&mut lines, "Capture is skipped if the response isn't JSON.");
        blank(&mut lines);

        section(&mut lines, "Modals");
        para(
            &mut lines,
            "Esc closes any modal (Add Request, Env editor, History, Help) without saving extras.",
        );

        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .alignment(Alignment::Left),
            inner,
        );

        // Footer
        let footer_y = area.y + area.height.saturating_sub(2);
        frame.render_widget(
            Paragraph::new("j/k scroll  ? or Esc close")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            Rect {
                x: area.x,
                y: footer_y,
                width: area.width,
                height: 1,
            },
        );
    }
}

fn section(lines: &mut Vec<Line>, title: &str) {
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
}

fn kv(lines: &mut Vec<Line>, key: &str, desc: &str) {
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{key:<28}"), Style::default().fg(Color::Yellow)),
        Span::raw(desc.to_string()),
    ]));
}

fn para(lines: &mut Vec<Line>, body: &str) {
    lines.push(Line::from(Span::raw(format!("  {body}"))));
}

fn blank(lines: &mut Vec<Line>) {
    lines.push(Line::from(""));
}
