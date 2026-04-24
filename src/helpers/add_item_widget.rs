use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Tabs as TabsWidget};
use ratatui::{symbols, Frame};

use crate::helpers::items::{Auth, ApiKeyLocation, Item, QueryParam, Request, RequestBody, RequestType};
use crate::helpers::text_input::TextInput;
use crate::helpers::body_editor::BodyEditor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Info,
    Auth,
    Body,
    Params,
    Headers,
}

impl Tab {
    const ALL: &'static [Tab] = &[
        Tab::Info,
        Tab::Auth,
        Tab::Body,
        Tab::Params,
        Tab::Headers,
    ];

    fn count() -> usize {
        Self::ALL.len()
    }

    fn titles() -> Vec<Line<'static>> {
        Self::ALL.iter().map(|t| Line::from(t.label())).collect()
    }

    fn label(self) -> &'static str {
        match self {
            Tab::Info => " Info ",
            Tab::Auth => " Auth ",
            Tab::Body => " Body ",
            Tab::Params => " Params ",
            Tab::Headers => " Headers ",
        }
    }
}

const METHODS: &[RequestType] = &[
    RequestType::Get,
    RequestType::Post,
    RequestType::Put,
    RequestType::Patch,
    RequestType::Delete,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusId {
    // Info tab
    MethodSelector,
    NameInput,
    UrlInput,
    // Auth tab
    AuthBearerToken,
    AuthBasicUser,
    AuthBasicPass,
    AuthApiKey,
    AuthApiValue,
    // Body tab
    BodyRaw,
    Param(usize, bool),
    // Headers tab (dynamic)
    Header(usize),
}


pub struct AddItemWidget {
    current_tab: usize,
    focused: Option<FocusId>,
    editing: bool,

    // Method selector
    method_index: usize,

    // Info tab inputs
    name_input: TextInput,
    url_input: TextInput,

    // Auth tab inputs
    auth_bearer_token: TextInput,
    auth_basic_user: TextInput,
    auth_basic_pass: TextInput,
    auth_api_key: TextInput,
    auth_api_value: TextInput,

    // Body tab
    body_editor: BodyEditor,

    // Params / Headers rows
    param_inputs: Vec<(TextInput, TextInput)>,
    header_inputs: Vec<(TextInput, TextInput)>,

    pub is_open: bool,
    /// Set to Some(item) when the user presses Enter to confirm.
    pub finished_item: Option<Item>,
}

impl AddItemWidget {
    pub fn new() -> Self {
        Self {
            current_tab: 0,
            focused: None,
            editing: false,
            method_index: 0,
            name_input: TextInput::new("Name"),
            url_input: TextInput::new("URL"),
            auth_bearer_token: TextInput::new("Bearer Token"),
            auth_basic_user: TextInput::new("Username"),
            auth_basic_pass: TextInput::new("Password"),
            auth_api_key: TextInput::new("API Key"),
            auth_api_value: TextInput::new("API Value"),
            body_editor: BodyEditor::new(),
            param_inputs: vec![Self::new_kv_row("Key 1", "Value 1")],
            header_inputs: vec![Self::new_kv_row("Header 1", "Value 1")],
            is_open: true,
            finished_item: None,
        }
    }

    fn new_kv_row(key_title: &str, val_title: &str) -> (TextInput, TextInput) {
        (TextInput::new(key_title.to_owned()), TextInput::new(val_title.to_owned()))
    }

    fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % Tab::count();
        self.reset_focus();
    }

    fn prev_tab(&mut self) {
        self.current_tab = (self.current_tab + Tab::count() - 1) % Tab::count();
        self.reset_focus();
    }

    fn reset_focus(&mut self) {
        self.focused = None;
        self.editing = false;
        self.disable_all_inputs();
    }

    fn disable_all_inputs(&mut self) {
        self.name_input.disable();
        self.url_input.disable();
        self.auth_bearer_token.disable();
        self.auth_basic_user.disable();
        self.auth_basic_pass.disable();
        self.auth_api_key.disable();
        self.auth_api_value.disable();
        self.body_editor.disable();
        for (k, v) in &mut self.param_inputs {
            k.disable();
            v.disable();
        }
        for (k, v) in &mut self.header_inputs {
            k.disable();
            v.disable();
        }
    }

    fn enable_focused_input(&mut self) {
        self.disable_all_inputs();
        match self.focused {
            Some(FocusId::NameInput) => self.name_input.enable(),
            Some(FocusId::UrlInput) => self.url_input.enable(),
            Some(FocusId::AuthBearerToken) => self.auth_bearer_token.enable(),
            Some(FocusId::AuthBasicUser) => self.auth_basic_user.enable(),
            Some(FocusId::AuthBasicPass) => self.auth_basic_pass.enable(),
            Some(FocusId::AuthApiKey) => self.auth_api_key.enable(),
            Some(FocusId::AuthApiValue) => self.auth_api_value.enable(),
            Some(FocusId::BodyRaw) => self.body_editor.enable(),
            Some(FocusId::Param(i, val_col)) => {
                if val_col {
                    if let Some((_, v)) = self.param_inputs.get_mut(i) { v.enable(); }
                } else {
                    if let Some((k, _)) = self.param_inputs.get_mut(i) { k.enable(); }
                }
            }
            Some(FocusId::Header(i)) => {
                if let Some((k, _)) = self.header_inputs.get_mut(i) { k.enable(); }
            }
            _ => {}
        }
    }

    fn forward_to_active_input(&mut self, event: &Event) {
        match self.focused {
            Some(FocusId::NameInput) => { self.name_input.handle_event(event); }
            Some(FocusId::UrlInput) => { self.url_input.handle_event(event); }
            Some(FocusId::AuthBearerToken) => { self.auth_bearer_token.handle_event(event); }
            Some(FocusId::AuthBasicUser) => { self.auth_basic_user.handle_event(event); }
            Some(FocusId::AuthBasicPass) => { self.auth_basic_pass.handle_event(event); }
            Some(FocusId::AuthApiKey) => { self.auth_api_key.handle_event(event); }
            Some(FocusId::AuthApiValue) => { self.auth_api_value.handle_event(event); }
            Some(FocusId::BodyRaw) => { self.body_editor.handle_event(event); }
            Some(FocusId::Param(i, val_col)) => {
                if val_col {
                    if let Some((_, v)) = self.param_inputs.get_mut(i) { v.handle_event(event); }
                } else {
                    if let Some((k, _)) = self.param_inputs.get_mut(i) { k.handle_event(event); }
                }
            }
            Some(FocusId::Header(i)) => {
                if let Some((k, _)) = self.header_inputs.get_mut(i) { k.handle_event(event); }
            }
            _ => {}
        }
    }

    fn focus_list(&self) -> Vec<FocusId> {
        match Tab::ALL[self.current_tab] {
            Tab::Info => vec![
                FocusId::MethodSelector,
                FocusId::NameInput,
                FocusId::UrlInput,
            ],
            Tab::Auth => vec![
                FocusId::AuthBearerToken,
                FocusId::AuthBasicUser,
                FocusId::AuthBasicPass,
                FocusId::AuthApiKey,
                FocusId::AuthApiValue,
            ],
            Tab::Body => vec![FocusId::BodyRaw],
            Tab::Params => (0..self.param_inputs.len()).flat_map(|i| [FocusId::Param(i, false), FocusId::Param(i, true)]).collect(),
            Tab::Headers => (0..self.header_inputs.len()).map(FocusId::Header).collect(),
        }
    }

    fn move_focus_down(&mut self) {
        let list = self.focus_list();
        if list.is_empty() { return; }
        self.focused = Some(match self.focused {
            None => list[0],
            Some(cur) => {
                let pos = list.iter().position(|f| *f == cur).unwrap_or(0);
                list[(pos + 1) % list.len()]
            }
        });
    }

    fn move_focus_up(&mut self) {
        let list = self.focus_list();
        if list.is_empty() { return; }
        self.focused = Some(match self.focused {
            None => *list.last().unwrap(),
            Some(cur) => {
                let pos = list.iter().position(|f| *f == cur).unwrap_or(0);
                list[(pos + list.len() - 1) % list.len()]
            }
        });
    }

    fn build_item(&mut self) -> Item {
        let method = METHODS[self.method_index].clone();
        let name   = self.name_input.value().to_string();
        let url    = self.url_input.value().to_string();

        let auth = {
            let bearer  = self.auth_bearer_token.value().to_string();
            let user    = self.auth_basic_user.value().to_string();
            let pass    = self.auth_basic_pass.value().to_string();
            let api_key = self.auth_api_key.value().to_string();
            let api_val = self.auth_api_value.value().to_string();

            if !bearer.is_empty() {
                Some(Auth::Bearer { token: bearer })
            } else if !user.is_empty() || !pass.is_empty() {
                Some(Auth::Basic { username: user, password: pass })
            } else if !api_key.is_empty() {
                Some(Auth::ApiKey {
                    key: api_key,
                    value: api_val,
                    location: ApiKeyLocation::Header,
                })
            } else {
                None
            }
        };

        let body = {
            let raw = self.body_editor.value();
            if raw.trim().is_empty() { None } else { Some(RequestBody::Raw(raw)) }
        };

        let params: Vec<QueryParam> = self.param_inputs.iter()
            .filter_map(|(k, v)| {
                let key = k.value().to_string();
                if key.is_empty() { None }
                else { Some(QueryParam { key, value: v.value().to_string(), enabled: true }) }
            })
            .collect();

        let headers: std::collections::HashMap<String, String> = self.header_inputs.iter()
            .filter_map(|(k, v)| {
                let key = k.value().to_string();
                if key.is_empty() { None }
                else { Some((key, v.value().to_string())) }
            })
            .collect();

        Item::Request(Request {
            name: if name.is_empty() { "Unnamed".into() } else { name },
            request_type: method,
            url,
            headers: if headers.is_empty() { None } else { Some(headers) },
            body,
            auth,
            params: if params.is_empty() { None } else { Some(params) },
        })
    }


    pub fn handle_event(&mut self, event: &Event) -> Result<bool, &'static str> {
        let Event::Key(key) = event else { return Ok(true) };
        if key.kind != KeyEventKind::Press { return Ok(true) }

        if self.editing {
            if key.code == KeyCode::Esc {
                self.editing = false;
                self.disable_all_inputs();
            } else {
                self.forward_to_active_input(event);
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_tab();
                } else {
                    self.next_tab();
                }
            }
            KeyCode::BackTab => self.prev_tab(),

            KeyCode::Char('j') | KeyCode::Down  => self.move_focus_down(),
            KeyCode::Char('k') | KeyCode::Up    => self.move_focus_up(),

            KeyCode::Char('e') => {
                if let Some(fid) = self.focused {
                    match fid {
                        FocusId::MethodSelector => {
                            self.method_index = (self.method_index + 1) % METHODS.len();
                        }
                        _ => {
                            self.editing = true;
                            self.enable_focused_input();
                        }
                    }
                }
            }

            KeyCode::Char('l') | KeyCode::Right
                if self.focused == Some(FocusId::MethodSelector) =>
            {
                self.method_index = (self.method_index + 1) % METHODS.len();
            }
            KeyCode::Char('h') | KeyCode::Left
                if self.focused == Some(FocusId::MethodSelector) =>
            {
                self.method_index = (self.method_index + METHODS.len() - 1) % METHODS.len();
            }

            KeyCode::Char('d') => {
                if let Some(FocusId::Param(idx, _)) = self.focused {
                    self.param_inputs.remove(idx);
                } else if let Some(FocusId::Header(idx)) = self.focused {
                    self.header_inputs.remove(idx);
                }
            }

            KeyCode::Char('a') => match Tab::ALL[self.current_tab] {
                Tab::Params => {
                    let idx = self.param_inputs.len();
                    self.param_inputs.push(Self::new_kv_row(
                        &format!("Key {}", idx + 1),
                        &format!("Value {}", idx + 1),
                    ));
                    self.focused = Some(FocusId::Param(idx, false));
                }
                Tab::Headers => {
                    let idx = self.header_inputs.len();
                    self.header_inputs.push(Self::new_kv_row(
                        &format!("Header {}", idx + 1),
                        &format!("Value {}", idx + 1),
                    ));
                    self.focused = Some(FocusId::Header(idx));
                }
                _ => {}
            },

            KeyCode::Enter => {
                let item = self.build_item();
                self.finished_item = Some(item);
                self.is_open = false;
            }

            KeyCode::Esc => {
                self.is_open = false;
            }

            _ => {}
        }

        Ok(true)
    }


    pub fn render_modal(&mut self, frame: &mut Frame, screen_area: Rect) {
        let modal_width  = screen_area.width.min(72);
        let modal_height = screen_area.height.min(32);
        let x = screen_area.x + (screen_area.width.saturating_sub(modal_width)) / 2;
        let y = screen_area.y + (screen_area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect { x, y, width: modal_width, height: modal_height };
        frame.render_widget(Clear, modal_area);
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Add Request "),
            modal_area,
        );

        let hint = if self.editing {
            "Esc: stop editing"
        } else if self.current_tab == Tab::Params as usize || self.current_tab == Tab::Headers as usize {
            "a: add param/header  d: delete"
        } else {
            "j/k: navigate  e: edit  Tab: switch tab  Enter: add  Esc: close"
        };
        let footer_y = modal_area.y + modal_area.height.saturating_sub(2);
        frame.render_widget(
            Paragraph::new(hint)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            Rect { x: modal_area.x, y: footer_y, width: modal_area.width, height: 1 },
        );

        let inner = modal_area.inner(Margin { vertical: 1, horizontal: 2 });

        let tab_row  = Rect { x: inner.x, y: inner.y,     width: inner.width, height: 1 };
        let divider  = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: 1 };
        let body     = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height.saturating_sub(2),
        };

        let tabs_widget = TabsWidget::new(Tab::titles())
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().yellow().on_black().bold())
            .select(self.current_tab)
            .divider(symbols::DOT)
            .padding("", "");
        frame.render_widget(tabs_widget, tab_row);

        frame.render_widget(
            Paragraph::new("─".repeat(inner.width as usize))
                .style(Style::default().fg(Color::DarkGray)),
            divider,
        );

        match Tab::ALL[self.current_tab] {
            Tab::Info    => self.render_info_tab(frame, body),
            Tab::Auth    => self.render_auth_tab(frame, body),
            Tab::Body    => self.render_body_tab(frame, body),
            Tab::Params  => self.render_kv_tab(frame, body, false),
            Tab::Headers => self.render_kv_tab(frame, body, true),
        }
    }

    fn render_info_tab(&self, frame: &mut Frame, area: Rect) {
        let constraints = [
            Constraint::Length(3), // method selector
            Constraint::Length(3), // name
            Constraint::Length(3), // url
        ];
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        self.render_method_selector(frame, chunks[0], self.focused == Some(FocusId::MethodSelector));
        Self::render_input_with_focus(frame, chunks[1], &self.name_input, self.focused == Some(FocusId::NameInput), self.editing);
        Self::render_input_with_focus(frame, chunks[2], &self.url_input,  self.focused == Some(FocusId::UrlInput),  self.editing);
    }

    fn render_auth_tab(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(area);

        Self::render_input_with_focus(frame, chunks[0], &self.auth_bearer_token, self.focused == Some(FocusId::AuthBearerToken), self.editing);
        Self::render_input_with_focus(frame, chunks[1], &self.auth_basic_user,   self.focused == Some(FocusId::AuthBasicUser),   self.editing);
        Self::render_input_with_focus(frame, chunks[2], &self.auth_basic_pass,   self.focused == Some(FocusId::AuthBasicPass),   self.editing);
        Self::render_input_with_focus(frame, chunks[3], &self.auth_api_key,      self.focused == Some(FocusId::AuthApiKey),      self.editing);
        Self::render_input_with_focus(frame, chunks[4], &self.auth_api_value,    self.focused == Some(FocusId::AuthApiValue),    self.editing);
    }

    fn render_body_tab(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focused == Some(FocusId::BodyRaw);
        self.body_editor.render(frame, area, is_focused, " Body ");
    }

    fn render_kv_tab(&self, frame: &mut Frame, area: Rect, is_headers: bool) {
        let rows = if is_headers { &self.header_inputs } else { &self.param_inputs };

        let list_area = Rect { height: area.height.saturating_sub(2), ..area };
        let mut y_offset = 0u16;
        for (i, (key_input, val_input)) in rows.iter().enumerate() {
            if y_offset + 3 > list_area.height { break; }
            let half = list_area.width / 2;
            if is_headers {
                let fid = FocusId::Header(i);
                let row_area = Rect {
                    x: list_area.x,
                    y: list_area.y + y_offset,
                    width: list_area.width,
                    height: 3,
                };
                Self::render_input_with_focus(frame, row_area, key_input, self.focused == Some(fid), self.editing);
            } else {
                let key_area = Rect {
                    x: list_area.x,
                    y: list_area.y + y_offset,
                    width: half,
                    height: 3,
                };
                let val_area = Rect {
                    x: list_area.x + half,
                    y: list_area.y + y_offset,
                    width: list_area.width - half,
                    height: 3,
                };
                Self::render_input_with_focus(frame, key_area, key_input, self.focused == Some(FocusId::Param(i, false)), self.editing);
                Self::render_input_with_focus(frame, val_area, val_input, self.focused == Some(FocusId::Param(i, true)), self.editing);
            }
            y_offset += 3;
        }
    }
    fn render_method_selector(&self, frame: &mut Frame, area: Rect, is_focused: bool) {
        let selected = &METHODS[self.method_index];
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Method (e/h/l) ")
            .border_style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let spans: Vec<Span> = METHODS.iter().enumerate().flat_map(|(i, m)| {
            let style = if m == selected {
                Style::default().fg(m.color()).bold().underlined()
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let mut v: Vec<Span> = vec![Span::styled(m.as_str(), style)];
            if i + 1 < METHODS.len() {
                v.push(Span::raw("  "));
            }
            v
        }).collect();

        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_input_with_focus(
        frame: &mut Frame,
        area: Rect,
        input: &TextInput,
        is_focused: bool,
        editing: bool,
    ) {
        input.render(frame, area);

        if is_focused && !editing {
            frame.render_widget(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::White))
                    .title(input.title.clone()),
                area,
            );
        }
    }
}
