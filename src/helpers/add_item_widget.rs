use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Tabs as TabsWidget};
use ratatui::{symbols, Frame};

use crate::helpers::items::RequestType;
use crate::helpers::text_input::TextInput;

pub enum Tabs {
    Info,
    Request,
}

pub enum SelectedInput {
    Name,
}

impl Tabs {
    pub fn get_tab(tab: usize) -> Self {
        match tab {
            0 => Tabs::Info,
            1 => Tabs::Request,
            _ => Tabs::Info,
        }
    }

    pub fn get_tabs() -> Vec<Tabs> {
        vec![Tabs::Info, Tabs::Request]
    }

    pub fn titles() -> Vec<Line<'static>> {
        vec![Line::from("Info"), Line::from("Request")]
    }
}

pub struct AddItemWidget {
    current_tab: usize,
    selected_input: Option<SelectedInput>,
    item_type: RequestType,
    name_input: TextInput,
    pub is_open: bool,
}

impl AddItemWidget {
    pub fn new() -> Self {
        Self {
            current_tab: 0,
            selected_input: None,
            item_type: RequestType::Get,
            name_input: TextInput::new("Name"),
            is_open: true,
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % Tabs::get_tabs().len();
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = (self.current_tab + Tabs::get_tabs().len() - 1) % Tabs::get_tabs().len();
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        self.name_input.handle_event(event);

        match event {
            Event::Key(key_event) => {
                if key_event.kind != KeyEventKind::Press {
                    return false;
                }

                match key_event.code {
                    KeyCode::Tab => {
                        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                            self.prev_tab();
                        } else {
                            self.next_tab();
                        }
                        return true;
                    }
                    KeyCode::BackTab => {
                        self.prev_tab();
                        return true;
                    }
                    KeyCode::Char('e') => {
                        self.name_input.enable();
                        self.selected_input = Some(SelectedInput::Name);
                        return true;
                    }
                    KeyCode::Esc => {
                        if let Some(_selected_input) = &self.selected_input {
                            self.name_input.disable();
                            self.selected_input = None;
                        } else {
                            self.is_open = false;
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        false
    }


    pub fn render_modal(&mut self, frame: &mut Frame, screen_area: Rect) {
        let modal_width = screen_area.width.min(70);
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

        let tabs = Tabs::get_tabs();
        let current_tab = &tabs[self.current_tab];
        let title = match current_tab {
            Tabs::Info => "Add Item",
            Tabs::Request => "Add Item",
        };

        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title),
            modal_area,
        );

        let footer_area = Rect {
            x: modal_area.x,
            y: modal_area.y + modal_area.height.saturating_sub(2),
            width: modal_area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new("Esc: close • Tab/Shift+Tab: switch")
                .alignment(Alignment::Center),
            footer_area,
        );

        let inner = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(inner);

        let tabs_widget = TabsWidget::new(Tabs::titles())
            .style(Color::White)
            .highlight_style(Style::default().yellow().on_black().bold())
            .select(self.current_tab)
            .divider(symbols::DOT)
            .padding(" ", " ");
        frame.render_widget(tabs_widget, areas[0]);

        self.name_input.render(frame, areas[1]);

        let body_title = match current_tab {
            Tabs::Info => "Info",
            Tabs::Request => "Request",
        };
        frame.render_widget(
            Paragraph::new(Line::from(format!("Tab: {body_title}")).alignment(Alignment::Center))
                .block(Block::bordered()),
            areas[2],
        );
    }

}
