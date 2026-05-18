use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType},
};
use ratatui_textarea::{CursorMove, DataCursor, TextArea};

/// Editor "mode" — affects which keystrokes get intercepted before being
/// forwarded to the underlying textarea.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Pass everything straight through to the textarea.
    #[default]
    Plain,
    /// Auto-close brackets/quotes, smart Enter inside `{}` and `[]`, Tab → 2
    /// spaces, indent-preserving newlines. Designed for JSON.
    Json,
}

const JSON_INDENT_WIDTH: usize = 2;

/// Multi-line text area used for raw/JSON request bodies and capture templates.
/// Wraps `ratatui_textarea::TextArea` with an enabled/disabled toggle, a
/// focus-aware border, and an optional [`EditorMode::Json`] mode that adds
/// bracket matching and smart indentation.
#[derive(Default)]
pub struct BodyEditor {
    textarea: TextArea<'static>,
    pub enabled: bool,
    pub mode: EditorMode,
}

impl std::fmt::Debug for BodyEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BodyEditor")
            .field("enabled", &self.enabled)
            .field("mode", &self.mode)
            .finish()
    }
}

impl Clone for BodyEditor {
    fn clone(&self) -> Self {
        let mut ta = TextArea::from(self.textarea.lines().to_vec());
        ta.set_style(self.textarea.style());
        Self {
            textarea: ta,
            enabled: self.enabled,
            mode: self.mode,
        }
    }
}

impl BodyEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        self.textarea = TextArea::from(lines);
    }

    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }
        if self.mode == EditorMode::Json
            && let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
            && self.try_handle_json_key(key)
        {
            return true;
        }
        use ratatui_textarea::Input;
        self.textarea.input(Input::from(event.clone()));
        true
    }

    /// Returns true if the JSON-aware path handled this key. Returning false
    /// falls through to the textarea's default behavior.
    fn try_handle_json_key(&mut self, key: &KeyEvent) -> bool {
        // Don't interfere with modifier combos — the textarea owns those
        // (Ctrl-A select all, etc.).
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }

        match key.code {
            KeyCode::Char('{') => self.insert_pair('{', '}'),
            KeyCode::Char('[') => self.insert_pair('[', ']'),
            KeyCode::Char('(') => self.insert_pair('(', ')'),
            KeyCode::Char(c) if matches!(c, '}' | ']' | ')') => self.step_over(c),
            KeyCode::Char('"') => self.handle_quote(),
            KeyCode::Enter => self.handle_newline(),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Tab => {
                self.textarea.insert_str(" ".repeat(JSON_INDENT_WIDTH));
                true
            }
            _ => false,
        }
    }

    fn insert_pair(&mut self, open: char, close: char) -> bool {
        let mut s = String::with_capacity(2);
        s.push(open);
        s.push(close);
        self.textarea.insert_str(&s);
        self.textarea.move_cursor(CursorMove::Back);
        true
    }

    /// If the next char already matches `c` (e.g. user typed `}` after we
    /// auto-inserted one), step the cursor past it instead of inserting.
    fn step_over(&mut self, c: char) -> bool {
        if self.next_char() == Some(c) {
            self.textarea.move_cursor(CursorMove::Forward);
            true
        } else {
            false
        }
    }

    fn handle_quote(&mut self) -> bool {
        // Step through a paired `"` we previously auto-inserted; otherwise
        // open a fresh pair.
        if self.next_char() == Some('"') {
            self.textarea.move_cursor(CursorMove::Forward);
        } else {
            self.textarea.insert_str("\"\"");
            self.textarea.move_cursor(CursorMove::Back);
        }
        true
    }

    fn handle_newline(&mut self) -> bool {
        let indent = self.current_indent();
        let prev = self.prev_char();
        let next = self.next_char();
        let opens_block = matches!(prev, Some('{') | Some('['));
        let between_pair = matches!(
            (prev, next),
            (Some('{'), Some('}')) | (Some('['), Some(']'))
        );

        if between_pair {
            // {<cursor>}  ->  {␊  <cursor>␊}
            let inner = " ".repeat(indent + JSON_INDENT_WIDTH);
            let outer = " ".repeat(indent);
            self.textarea.insert_newline();
            self.textarea.insert_str(&inner);
            self.textarea.insert_newline();
            self.textarea.insert_str(&outer);
            self.textarea.move_cursor(CursorMove::Up);
            self.textarea.move_cursor(CursorMove::End);
            true
        } else if opens_block {
            // {<cursor>...  ->  {␊  <cursor>...
            let inner = " ".repeat(indent + JSON_INDENT_WIDTH);
            self.textarea.insert_newline();
            self.textarea.insert_str(&inner);
            true
        } else if indent > 0 {
            // Plain newline that preserves the current line's leading spaces.
            self.textarea.insert_newline();
            self.textarea.insert_str(" ".repeat(indent));
            true
        } else {
            false
        }
    }

    fn handle_backspace(&mut self) -> bool {
        // Delete an empty auto-inserted pair: `{|}` -> `|`, `"|"` -> `|`, etc.
        let pair = matches!(
            (self.prev_char(), self.next_char()),
            (Some('{'), Some('}'))
                | (Some('['), Some(']'))
                | (Some('('), Some(')'))
                | (Some('"'), Some('"'))
        );
        if pair {
            self.textarea.delete_next_char();
            self.textarea.delete_char();
            true
        } else {
            false
        }
    }

    fn next_char(&self) -> Option<char> {
        let DataCursor(row, col) = self.textarea.cursor();
        self.textarea
            .lines()
            .get(row)
            .and_then(|line| line.chars().nth(col))
    }

    fn prev_char(&self) -> Option<char> {
        let DataCursor(row, col) = self.textarea.cursor();
        if col == 0 {
            return None;
        }
        self.textarea
            .lines()
            .get(row)
            .and_then(|line| line.chars().nth(col - 1))
    }

    fn current_indent(&self) -> usize {
        let DataCursor(row, _) = self.textarea.cursor();
        self.textarea
            .lines()
            .get(row)
            .map(|l| l.chars().take_while(|c| *c == ' ').count())
            .unwrap_or(0)
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, title: &str) {
        let border_style = if self.enabled {
            Style::default().fg(Color::Yellow)
        } else if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title.to_owned())
            .border_style(border_style);

        self.textarea.set_block(block);
        self.textarea.set_cursor_style(if self.enabled {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default()
        });

        frame.render_widget(&self.textarea, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(c: char) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn key_code(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    fn json_editor() -> BodyEditor {
        let mut e = BodyEditor::new();
        e.enable();
        e.set_mode(EditorMode::Json);
        e
    }

    fn type_str(e: &mut BodyEditor, s: &str) {
        for c in s.chars() {
            e.handle_event(&key(c));
        }
    }

    #[test]
    fn open_brace_auto_closes() {
        let mut e = json_editor();
        e.handle_event(&key('{'));
        assert_eq!(e.value(), "{}");
        // Cursor is between the braces — typing now lands inside.
        e.handle_event(&key('a'));
        assert_eq!(e.value(), "{a}");
    }

    #[test]
    fn open_bracket_and_paren_auto_close() {
        let mut e = json_editor();
        e.handle_event(&key('['));
        assert_eq!(e.value(), "[]");
        let mut e2 = json_editor();
        e2.handle_event(&key('('));
        assert_eq!(e2.value(), "()");
    }

    #[test]
    fn quote_auto_closes_and_steps_over() {
        let mut e = json_editor();
        e.handle_event(&key('"'));
        assert_eq!(e.value(), "\"\"");
        // Typing `"` again should step over the closer, not insert another pair.
        e.handle_event(&key('"'));
        assert_eq!(e.value(), "\"\"");
    }

    #[test]
    fn typing_closer_after_autoclose_steps_over() {
        let mut e = json_editor();
        type_str(&mut e, "{");
        // Cursor between {|} -> typing } should step over, not duplicate.
        e.handle_event(&key('}'));
        assert_eq!(e.value(), "{}");
    }

    #[test]
    fn backspace_inside_empty_pair_removes_both() {
        let mut e = json_editor();
        type_str(&mut e, "{");
        e.handle_event(&key_code(KeyCode::Backspace));
        assert_eq!(e.value(), "");
    }

    #[test]
    fn enter_inside_braces_expands_block() {
        let mut e = json_editor();
        type_str(&mut e, "{");
        e.handle_event(&key_code(KeyCode::Enter));
        // After Enter inside {|}, expect:
        // {
        //   |
        // }
        assert_eq!(e.value(), "{\n  \n}");
    }

    #[test]
    fn enter_preserves_indent() {
        let mut e = json_editor();
        e.set_text("    hello");
        // Move cursor to end of line.
        e.textarea.move_cursor(CursorMove::End);
        e.handle_event(&key_code(KeyCode::Enter));
        assert_eq!(e.value(), "    hello\n    ");
    }

    #[test]
    fn tab_inserts_two_spaces() {
        let mut e = json_editor();
        e.handle_event(&key_code(KeyCode::Tab));
        assert_eq!(e.value(), "  ");
    }

    #[test]
    fn plain_mode_does_not_intercept() {
        let mut e = BodyEditor::new();
        e.enable();
        // mode defaults to Plain.
        e.handle_event(&key('{'));
        assert_eq!(e.value(), "{");
    }
}
