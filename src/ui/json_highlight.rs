use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};

/// Tokenize a pretty-printed JSON string into a ratatui `Text` with syntax
/// highlighting. Falls back to plain text on any unexpected input.
pub fn highlight_json(input: &str) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();

    #[derive(Clone, Copy)]
    enum Ctx {
        Object,
        Array,
    }

    let mut stack: Vec<Ctx> = Vec::new();
    // True when the next string token is an object key.
    let mut next_is_key = false;

    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        match chars[i] {
            '\n' => {
                lines.push(Line::from(std::mem::take(&mut spans)));
                i += 1;
            }

            '"' => {
                let is_key = next_is_key;
                next_is_key = false;

                let mut s = String::from('"');
                i += 1;
                let mut escaped = false;
                while i < len {
                    let c = chars[i];
                    s.push(c);
                    i += 1;
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
                let color = if is_key { Color::Cyan } else { Color::Green };
                spans.push(Span::styled(s, Style::default().fg(color)));
            }

            '{' => {
                stack.push(Ctx::Object);
                next_is_key = true;
                spans.push(Span::styled("{", Style::default().fg(Color::White)));
                i += 1;
            }
            '}' => {
                stack.pop();
                next_is_key = false;
                spans.push(Span::styled("}", Style::default().fg(Color::White)));
                i += 1;
            }
            '[' => {
                stack.push(Ctx::Array);
                next_is_key = false;
                spans.push(Span::styled("[", Style::default().fg(Color::White)));
                i += 1;
            }
            ']' => {
                stack.pop();
                spans.push(Span::styled("]", Style::default().fg(Color::White)));
                i += 1;
            }

            ':' => {
                // Next string will be a value, not a key.
                next_is_key = false;
                spans.push(Span::styled(":", Style::default().fg(Color::DarkGray)));
                i += 1;
            }

            ',' => {
                if matches!(stack.last(), Some(Ctx::Object)) {
                    next_is_key = true;
                }
                spans.push(Span::styled(",", Style::default().fg(Color::DarkGray)));
                i += 1;
            }

            '-' | '0'..='9' => {
                let start = i;
                while i < len && matches!(chars[i], '0'..='9' | '.' | '-' | '+' | 'e' | 'E') {
                    i += 1;
                }
                let num: String = chars[start..i].iter().collect();
                spans.push(Span::styled(num, Style::default().fg(Color::Yellow)));
            }

            't' | 'f' | 'n' => {
                let start = i;
                while i < len && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let kw: String = chars[start..i].iter().collect();
                let color = match kw.as_str() {
                    "true" | "false" => Color::Magenta,
                    "null" => Color::Red,
                    _ => Color::White,
                };
                spans.push(Span::styled(kw, Style::default().fg(color)));
            }

            // Whitespace — preserve as plain text (indentation, spaces after ':').
            c if c.is_ascii_whitespace() => {
                let start = i;
                while i < len && chars[i].is_ascii_whitespace() && chars[i] != '\n' {
                    i += 1;
                }
                let ws: String = chars[start..i].iter().collect();
                spans.push(Span::raw(ws));
            }

            c => {
                spans.push(Span::raw(c.to_string()));
                i += 1;
            }
        }
    }

    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }

    Text::from(lines)
}
