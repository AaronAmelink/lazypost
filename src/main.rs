mod model;
mod net;
mod config;
mod logic;
mod ui;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind,
    KeyModifiers,
};
use config::env_config::EnvConfig;
use ui::environment_editor::EnvironmentEditor;
use ui::help_overlay::HelpOverlay;
use config::history::{History, HistoryAction};
use net::http_client::{self, ExecutedResponse, HttpError};
use model::items::{ConfigFolder, Item, Request, RequestType};
use ui::request_editor::RequestEditor;
use ui::response_view::ResponseView;
use ui::sidebar::Sidebar;
use config::workspace::WorkspaceConfig;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use std::path::Path;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::ui::add_item_widget::AddItemWidget;

const CONFIG_PATH: &str = "workspace.json";
const ENV_PATH: &str = "env.json";

pub enum AppMsg {
    ResponseReady(Result<ExecutedResponse, HttpError>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Sidebar,
    Editor,
    Response,
}

impl Pane {
    fn next(self) -> Self {
        match self {
            Pane::Sidebar => Pane::Editor,
            Pane::Editor => Pane::Response,
            Pane::Response => Pane::Sidebar,
        }
    }
    fn prev(self) -> Self {
        match self {
            Pane::Sidebar => Pane::Response,
            Pane::Editor => Pane::Sidebar,
            Pane::Response => Pane::Editor,
        }
    }
}

struct App {
    sidebar: Sidebar,
    add_item_widget: Option<AddItemWidget>,
    config: WorkspaceConfig,
    env_config: EnvConfig,
    runtime: Runtime,
    tx: UnboundedSender<AppMsg>,
    rx: UnboundedReceiver<AppMsg>,
    current_editor: Option<RequestEditor>,
    editing_path: Option<Vec<usize>>,
    focus_pane: Pane,
    save_status: Option<String>,
    response_view: ResponseView,
    last_response: Option<ExecutedResponse>,
    in_flight: bool,
    spinner_tick: u8,
    last_spin: std::time::Instant,
    env_editor: Option<EnvironmentEditor>,
    history: History,
    history_open: bool,
    last_sent_request: Option<Request>,
    help: HelpOverlay,
}

impl App {
    fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let (tx, rx) = unbounded_channel();
        Self {
            sidebar: Sidebar::new(default_items()),
            add_item_widget: None,
            config: WorkspaceConfig::new_empty(),
            env_config: EnvConfig::load(Path::new(ENV_PATH)),
            runtime,
            tx,
            rx,
            current_editor: None,
            editing_path: None,
            focus_pane: Pane::Sidebar,
            save_status: None,
            response_view: ResponseView::new(),
            last_response: None,
            in_flight: false,
            spinner_tick: 0,
            last_spin: std::time::Instant::now(),
            env_editor: None,
            history: History::load(Path::new(".")),
            history_open: false,
            last_sent_request: None,
            help: HelpOverlay::new(),
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMsg::ResponseReady(res) => {
                    self.in_flight = false;
                    match res {
                        Ok(r) => {
                            self.response_view.set_response(Some(&r));
                            if let Some(req) = self.last_sent_request.take() {
                                self.run_captures(&req, &r);
                                self.history.push(req, &r);
                            }
                            self.last_response = Some(r);
                        }
                        Err(e) => {
                            self.response_view.set_error(&e);
                            self.last_response = None;
                            self.last_sent_request = None;
                        }
                    }
                }
            }
        }
        // Advance spinner roughly every 100ms while a request is in flight.
        if self.in_flight && self.last_spin.elapsed() > Duration::from_millis(100) {
            self.spinner_tick = self.spinner_tick.wrapping_add(1);
            self.last_spin = std::time::Instant::now();
        }
    }

    fn run_captures(&mut self, req: &Request, resp: &ExecutedResponse) {
        let Some(template_str) = req.capture.as_ref() else {
            return;
        };
        let template: serde_json::Value = match serde_json::from_str(template_str) {
            Ok(v) => v,
            Err(e) => {
                self.save_status = Some(format!("capture template invalid JSON: {e}"));
                return;
            }
        };
        let actual: serde_json::Value = match serde_json::from_slice(&resp.body) {
            Ok(v) => v,
            Err(_) => {
                self.save_status = Some("capture skipped: response is not JSON".to_string());
                return;
            }
        };
        let pairs = logic::capture::extract_captures(&template, &actual);
        if pairs.is_empty() {
            self.save_status = Some("capture: nothing matched".to_string());
            return;
        }
        let names: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
        for (k, v) in pairs {
            self.env_config.data.variables.insert(k, v);
        }
        let _ = self.env_config.save();
        self.save_status = Some(format!("captured: {}", names.join(", ")));
    }

    fn send_current_request(&mut self) {
        if self.in_flight {
            return;
        }
        // Prefer the in-memory editor state (so unsaved edits are sent too).
        let req = match self.current_editor.as_mut() {
            Some(editor) => match editor.to_request() {
                Ok(r) => r,
                Err(_) => {
                    self.save_status = Some("invalid editor state".to_string());
                    return;
                }
            },
            None => return,
        };
        let vars = self.env_config.data.variables.clone();
        let tx = self.tx.clone();
        self.in_flight = true;
        self.spinner_tick = 0;
        self.last_spin = std::time::Instant::now();
        self.response_view.set_response(None);
        self.last_response = None;
        let req_for_history = req.clone();
        self.last_sent_request = Some(req_for_history);
        self.runtime.spawn(async move {
            let res = http_client::execute(req, vars).await;
            let _ = tx.send(AppMsg::ResponseReady(res));
        });
    }

    fn sync_editor_with_selection(&mut self) {
        let path = self.sidebar.selected_path.clone();
        let is_same = self.editing_path.as_ref() == Some(&path);
        if is_same {
            return;
        }

        // Selection changed: discard previous editor (silently drops unsaved edits).
        // Users explicitly save with Ctrl-S before navigating away.
        match self.sidebar.item_at(&path) {
            Some(Item::Request(req)) => {
                self.current_editor = Some(RequestEditor::from_request(req));
                self.editing_path = Some(path);
            }
            _ => {
                self.current_editor = None;
                self.editing_path = None;
            }
        }
    }

    fn save_current_editor(&mut self) {
        let Some(editor) = self.current_editor.as_mut() else {
            return;
        };
        let Some(path) = self.editing_path.clone() else {
            return;
        };
        match editor.to_request() {
            Ok(req) => {
                if ui::sidebar::replace_request_at(&mut self.sidebar.items, &path, req).is_ok()
                {
                    let _ = WorkspaceConfig::save_items_to_file(
                        self.sidebar.items.clone(),
                        Path::new(CONFIG_PATH),
                    );
                    self.save_status = Some("saved".to_string());
                }
            }
            Err(_) => {
                // editor.validation_error is now populated; render() shows it.
                self.save_status = Some("save failed (see editor)".to_string());
            }
        }
    }

    /// Auto-save without setting a status — used on focus/selection changes
    /// so saves feel implicit (lazygit-style).
    fn save_current_editor_silent(&mut self) {
        let Some(editor) = self.current_editor.as_mut() else {
            return;
        };
        let Some(path) = self.editing_path.clone() else {
            return;
        };
        if let Ok(req) = editor.to_request()
            && ui::sidebar::replace_request_at(&mut self.sidebar.items, &path, req).is_ok()
        {
            let _ = WorkspaceConfig::save_items_to_file(
                self.sidebar.items.clone(),
                Path::new(CONFIG_PATH),
            );
        }
        // If to_request errored, validation_error is set on the editor and the
        // user will see it on re-focus. Don't block navigation.
    }

    fn handle_events(&mut self, event: &Event) -> Result<bool, &'static str> {
        if matches!(event, Event::Paste(_)) {
            if let Some(widget) = &mut self.add_item_widget {
                if widget.is_editing() {
                    widget.handle_event(event)?;
                    return Ok(true);
                }
            }
            if let Some(editor) = &mut self.current_editor {
                if self.focus_pane == Pane::Editor && editor.is_editing() {
                    editor.handle_event(event);
                    return Ok(true);
                }
            }
        }

        let Event::Key(key) = event else {
            return Ok(true);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        // Help overlay trumps everything.
        if self.help.open {
            self.help.handle_event(event);
            return Ok(true);
        }

        // History overlay trumps everything.
        if self.history_open {
            match self.history.handle_event(event) {
                HistoryAction::None => {}
                HistoryAction::Close => {
                    self.history_open = false;
                }
                HistoryAction::Restore => {
                    if let Some(entry) = self.history.selected_entry() {
                        self.current_editor =
                            Some(RequestEditor::from_request(&entry.request_snapshot));
                        self.editing_path = None; // restored item is detached
                        let resp = entry.response.to_response();
                        self.response_view.set_response(Some(&resp));
                        self.last_response = Some(resp);
                        self.history_open = false;
                        self.focus_pane = Pane::Editor;
                    }
                }
            }
            return Ok(true);
        }

        // Env editor modal trumps everything else.
        if let Some(editor) = &mut self.env_editor {
            editor.handle_event(event);
            if !editor.is_open {
                if editor.finished {
                    self.env_config.data.variables = editor.collect();
                    let _ = self.env_config.save();
                }
                self.env_editor = None;
            }
            return Ok(true);
        }

        // Modal trumps everything.
        if let Some(widget) = &mut self.add_item_widget {
            widget.handle_event(event)?;
            if !widget.is_open {
                let finished = widget.finished_item.take();
                self.add_item_widget = None;
                if let Some(item) = finished {
                    let insert_path = self.sidebar.selected_path.clone();
                    if let Ok(new_path) = self.sidebar.add_item(insert_path, item) {
                        self.sidebar.selected_path = new_path;
                    }
                    let _ = WorkspaceConfig::save_items_to_file(
                        self.sidebar.items.clone(),
                        Path::new(CONFIG_PATH),
                    );
                    self.editing_path = None;
                    self.sync_editor_with_selection();
                }
            }
            return Ok(true);
        }

        // Global hotkeys (skip when a field is being actively typed into).
        let editor_typing = self
            .current_editor
            .as_ref()
            .map(|e| e.is_editing())
            .unwrap_or(false);
        if !editor_typing && !key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // lazygit-style pane cycling
                KeyCode::Tab => {
                    self.save_current_editor_silent();
                    self.focus_pane = self.focus_pane.next();
                    return Ok(true);
                }
                KeyCode::BackTab => {
                    self.save_current_editor_silent();
                    self.focus_pane = self.focus_pane.prev();
                    return Ok(true);
                }
                // Direct pane jumps
                KeyCode::Char('1') => {
                    self.save_current_editor_silent();
                    self.focus_pane = Pane::Sidebar;
                    return Ok(true);
                }
                KeyCode::Char('2') => {
                    self.focus_pane = Pane::Editor;
                    return Ok(true);
                }
                KeyCode::Char('3') => {
                    self.save_current_editor_silent();
                    self.focus_pane = Pane::Response;
                    return Ok(true);
                }
                // 'w' (vim-style :w) saves; also 'S' as backup. Ctrl-S avoided
                // because terminal XOFF often eats it.
                KeyCode::Char('w') | KeyCode::Char('S') => {
                    self.save_current_editor();
                    return Ok(true);
                }
                // Actions
                KeyCode::Char('n') => {
                    self.add_item_widget = Some(AddItemWidget::new());
                    return Ok(true);
                }
                KeyCode::Char('E') => {
                    self.env_editor = Some(EnvironmentEditor::new(
                        self.env_config.data.variables.clone(),
                    ));
                    return Ok(true);
                }
                KeyCode::Char('H') => {
                    self.history_open = true;
                    return Ok(true);
                }
                KeyCode::Char('s') if self.focus_pane != Pane::Response => {
                    self.send_current_request();
                    return Ok(true);
                }
                KeyCode::Char('?') => {
                    self.help.toggle();
                    return Ok(true);
                }
                KeyCode::Char('q') => {
                    return Ok(false);
                }
                _ => {}
            }
        }

        // Route to focused pane.
        match self.focus_pane {
            Pane::Sidebar => {
                let prev_path = self.sidebar.selected_path.clone();
                let is_delete = matches!(key.code, KeyCode::Char('d'));
                self.sidebar.handle_event(event)?;
                if self.sidebar.selected_path != prev_path || is_delete {
                    // Auto-save the editor we're leaving (lazygit-style).
                    self.save_current_editor_silent();
                    self.save_status = None;
                    self.editing_path = None;
                    self.sync_editor_with_selection();
                }
            }
            Pane::Editor => {
                if let Some(editor) = self.current_editor.as_mut() {
                    let was_editing = editor.is_editing();
                    editor.handle_event(event);
                    let now_editing = editor.is_editing();
                    // Save after any structural change (method, auth type, tab, etc.)
                    // and when exiting a text field. Don't save while actively typing
                    // or the moment a text field is first opened.
                    if was_editing || !now_editing {
                        self.save_current_editor_silent();
                    }
                }
            }
            Pane::Response => {
                self.response_view.handle_event(event);
            }
        }

        Ok(true)
    }
}

fn default_items() -> Vec<Item> {
    let req = |name: &str, request_type: RequestType| {
        Item::Request(Request {
            name: name.into(),
            request_type,
            url: String::new(),
            headers: None,
            body: None,
            auth: None,
            params: None,
            url_vars: None,
            capture: None,
        })
    };

    vec![
        req("Get Users", RequestType::Get),
        req("Create User", RequestType::Post),
        req("Update User", RequestType::Put),
        req("Delete User", RequestType::Delete),
        req("Get Posts", RequestType::Get),
        Item::Folder(ConfigFolder {
            name: "User Actions".into(),
            items: vec![
                Item::Folder(ConfigFolder {
                    name: "Nested Folder".into(),
                    items: vec![req("Delete User", RequestType::Delete)],
                }),
                Item::Folder(ConfigFolder {
                    name: "Another Nested".into(),
                    items: vec![req("Delete User", RequestType::Delete)],
                }),
            ],
        }),
    ]
}

fn main() {
    let mut app = App::new();
    app.config = WorkspaceConfig::create_from_file(Path::new(CONFIG_PATH))
        .unwrap_or_else(|_| WorkspaceConfig::new_empty());

    if app.config.data.items.is_empty() {
        app.config.data.items = default_items();
        let _ = app.config.save();
    } else {
        app.sidebar.items = app.config.data.items.clone();
    }

    app.sync_editor_with_selection();

    let _ = crossterm::execute!(std::io::stdout(), EnableBracketedPaste);
    ratatui::run(|terminal| {
        loop {
            let _ = terminal.draw(|frame| render(frame, &mut app));
            app.drain_messages();
            if !tick(&mut app).unwrap_or(false) {
                break;
            }
        }
    });
    let _ = crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
}

fn tick(app: &mut App) -> Result<bool, &'static str> {
    // Wake every ~50ms so async responses and spinners can advance even
    // without keypresses.
    let ready = event::poll(Duration::from_millis(50)).map_err(|_| "poll failed")?;
    if !ready {
        return Ok(true);
    }
    let event = event::read().map_err(|_| "read failed")?;
    app.handle_events(&event)
}

fn render(frame: &mut ratatui::Frame, app: &mut App) {
    // Carve off a 1-row status bar at the bottom.
    let [main_area, status_bar] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let [left, middle, right] = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .areas(main_area);

    // Sidebar
    let sidebar_border_style = pane_border_style(app.focus_pane == Pane::Sidebar);
    frame.render_widget(
        Block::bordered()
            .title("[1] Endpoints")
            .border_style(sidebar_border_style),
        left,
    );
    frame.render_widget(
        app.sidebar.clone(),
        left.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );

    // Request editor (middle pane)
    let editor_border_style = pane_border_style(app.focus_pane == Pane::Editor);
    frame.render_widget(
        Block::bordered()
            .title("[2] Request")
            .border_style(editor_border_style),
        middle,
    );
    let inner_middle = middle.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    if let Some(editor) = app.current_editor.as_mut() {
        editor.render(frame, inner_middle, app.focus_pane == Pane::Editor, None);
    } else {
        frame.render_widget(
            Paragraph::new("Select a request from the sidebar to edit it.")
                .style(Style::default().fg(Color::DarkGray)),
            inner_middle,
        );
    }

    // Response (right pane)
    let response_border_style = pane_border_style(app.focus_pane == Pane::Response);
    frame.render_widget(
        Block::bordered()
            .title("[3] Response")
            .border_style(response_border_style),
        right,
    );
    let inner_right = right.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    app.response_view.render(
        frame,
        inner_right,
        app.last_response.as_ref(),
        app.in_flight,
        app.spinner_tick,
    );

    // Status bar
    let mut spans = vec![
        Span::styled(
            format!(" {} ", pane_label(app.focus_pane)),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw("  "),
    ];
    let var_count = app.env_config.data.variables.len();
    spans.push(Span::styled(
        format!("vars: {var_count}"),
        Style::default().fg(Color::Magenta),
    ));
    spans.push(Span::raw("  "));
    if let Some(item) = &app.sidebar.clipboard {
        let label = match item {
            Item::Request(r) => format!("cut: \"{}\"", r.name),
            Item::Folder(f) => format!("cut: \"{}\"", f.name),
        };
        spans.push(Span::styled(label, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw("  "));
    }
    if let Some(status) = &app.save_status {
        spans.push(Span::styled(
            status.clone(),
            Style::default().fg(Color::Green),
        ));
        spans.push(Span::raw("  "));
    }
    let pane_hint = match app.focus_pane {
        Pane::Sidebar => "j/k nav  x cut  p paste  d del  n new  Enter folder",
        Pane::Editor  => "w save  s send  [/] tab  e edit  Esc stop",
        Pane::Response => "j/k scroll  [/] body/headers",
    };
    spans.push(Span::styled(pane_hint, Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        "   Tab/1-3 panes  E env  H history  ? help  q quit",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset)),
        status_bar,
    );

    if let Some(widget) = &mut app.add_item_widget {
        widget.render_modal(frame, frame.area());
    }
    if let Some(editor) = &app.env_editor {
        editor.render_modal(frame, frame.area());
    }
    if app.history_open {
        app.history.render(frame, frame.area());
    }
    if app.help.open {
        app.help.render(frame, frame.area());
    }
}

fn pane_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn pane_label(p: Pane) -> &'static str {
    match p {
        Pane::Sidebar => "SIDEBAR",
        Pane::Editor => "EDITOR",
        Pane::Response => "RESPONSE",
    }
}
