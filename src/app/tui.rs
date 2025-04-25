// app/tui.rs
// Terminal User Interface logic and state

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event::{self, Event, KeyCode};
use std::io;
pub use super::tui_export::tui_export_flow;

pub enum TuiStep {
    Welcome,
    ChooseDbType,
    EnterHost,
    EnterPort,
    EnterDbName,
    EnterUsername,
    EnterPassword,
    EnterSchema,
    Confirm,
    Progress,
    Done(String),
}

pub struct TuiState {
    pub step: TuiStep,
    pub db_type_index: usize,
    pub db_types: Vec<&'static str>,
    pub host: String,
    pub port: String,
    pub dbname: String,
    pub username: String,
    pub password: String,
    pub schema: String,
    pub connection_string: String,
    pub input_buffer: String,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            step: TuiStep::Welcome,
            db_type_index: 0,
            db_types: vec!["sqlite", "postgres", "mysql"],
            host: String::new(),
            port: String::new(),
            dbname: String::new(),
            username: String::new(),
            password: String::new(),
            schema: String::new(),
            connection_string: String::new(),
            input_buffer: String::new(),
        }
    }
}

pub async fn run_tui() -> io::Result<()> {
    // Clear the terminal before starting TUI
    crossterm::execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::All), crossterm::cursor::MoveTo(0, 0))?;
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = ratatui::backend::CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = TuiState::default();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            match &state.step {
                TuiStep::Welcome => {
                    let block = Block::default().title("DBMetaExporter").borders(Borders::ALL);
                    let text = Paragraph::new("Welcome! Press any key to begin.").block(block);
                    f.render_widget(text, size);
                }
                TuiStep::ChooseDbType => {
                    let items: Vec<ListItem> = state.db_types.iter().enumerate().map(|(i, &db)| {
                        let style = if i == state.db_type_index {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        ListItem::new(db).style(style)
                    }).collect();
                    let list = List::new(items)
                        .block(Block::default().title("Select Database Type (↑/↓, Enter)").borders(Borders::ALL));
                    f.render_widget(list, size);
                }
                TuiStep::EnterHost => {
                    let block = Block::default().title("Enter Host (e.g. localhost)").borders(Borders::ALL);
                    let text = Paragraph::new(state.input_buffer.as_str()).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::EnterPort => {
                    let block = Block::default().title("Enter Port (e.g. 5432)").borders(Borders::ALL);
                    let text = Paragraph::new(state.input_buffer.as_str()).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::EnterDbName => {
                    let block = Block::default().title("Enter Database Name (or SQLite file path)").borders(Borders::ALL);
                    let text = Paragraph::new(state.input_buffer.as_str()).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::EnterUsername => {
                    let block = Block::default().title("Enter Username").borders(Borders::ALL);
                    let text = Paragraph::new(state.input_buffer.as_str()).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::EnterPassword => {
                    let block = Block::default().title("Enter Password (hidden)").borders(Borders::ALL);
                    let hidden = "*".repeat(state.input_buffer.len());
                    let text = Paragraph::new(hidden).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::EnterSchema => {
                    let block = Block::default().title("Enter Schema/Database (optional)").borders(Borders::ALL);
                    let text = Paragraph::new(state.input_buffer.as_str()).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::Confirm => {
                    let block = Block::default().title("Confirm").borders(Borders::ALL);
                    let text = Paragraph::new(format!(
                        "DB: {}\nConn: {}\nUser: {}\nSchema: {}\nPress Enter to Export, Esc to Cancel",
                        state.db_types[state.db_type_index],
                        state.connection_string,
                        state.username,
                        state.schema
                    )).block(block);
                    f.render_widget(text, size);
                }
                TuiStep::Progress => {
                    let block = Block::default().title("Exporting...").borders(Borders::ALL);
                    let text = Paragraph::new("Please wait...").block(block);
                    f.render_widget(text, size);
                }
                TuiStep::Done(msg) => {
                    let block = Block::default().title("Done").borders(Borders::ALL);
                    let text = Paragraph::new(msg.clone()).block(block);
                    f.render_widget(text, size);
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match &mut state.step {
                    TuiStep::Welcome => {
                        state.step = TuiStep::ChooseDbType;
                    }
                    TuiStep::ChooseDbType => match key.code {
                        KeyCode::Up => {
                            if state.db_type_index > 0 {
                                state.db_type_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if state.db_type_index + 1 < state.db_types.len() {
                                state.db_type_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            let db = state.db_types[state.db_type_index];
                            match db {
                                "sqlite" => state.step = TuiStep::EnterDbName,
                                _ => state.step = TuiStep::EnterHost,
                            }
                            state.input_buffer.clear();
                        }
                        KeyCode::Esc => return Ok(()),
                        _ => {}
                    },
                    TuiStep::EnterHost => match key.code {
                        KeyCode::Enter => {
                            state.host = state.input_buffer.clone();
                            state.step = TuiStep::EnterPort;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => { state.step = TuiStep::ChooseDbType; },
                        _ => {}
                    },
                    TuiStep::EnterPort => match key.code {
                        KeyCode::Enter => {
                            state.port = state.input_buffer.clone();
                            state.step = TuiStep::EnterDbName;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => { state.step = TuiStep::EnterHost; },
                        _ => {}
                    },
                    TuiStep::EnterDbName => match key.code {
                        KeyCode::Enter => {
                            state.dbname = state.input_buffer.clone();
                            let db = state.db_types[state.db_type_index];
                            if db == "sqlite" {
                                state.build_connection_string();
                                state.step = TuiStep::Confirm;
                            } else {
                                state.step = TuiStep::EnterUsername;
                            }
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => {
                            let db = state.db_types[state.db_type_index];
                            if db == "sqlite" {
                                state.step = TuiStep::ChooseDbType;
                            } else {
                                state.step = TuiStep::EnterPort;
                            }
                        },
                        _ => {}
                    },
                    TuiStep::EnterUsername => match key.code {
                        KeyCode::Enter => {
                            state.username = state.input_buffer.clone();
                            state.step = TuiStep::EnterPassword;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => { state.step = TuiStep::EnterDbName; },
                        _ => {}
                    },
                    TuiStep::EnterPassword => match key.code {
                        KeyCode::Enter => {
                            state.password = state.input_buffer.clone();
                            state.step = TuiStep::EnterSchema;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => { state.step = TuiStep::EnterUsername; },
                        _ => {}
                    },
                    TuiStep::EnterSchema => match key.code {
                        KeyCode::Enter => {
                            state.schema = state.input_buffer.clone();
                            state.build_connection_string();
                            state.step = TuiStep::Confirm;
                            state.input_buffer.clear();
                        }
                        KeyCode::Char(c) => state.input_buffer.push(c),
                        KeyCode::Backspace => { state.input_buffer.pop(); },
                        KeyCode::Esc => {
                            let db = state.db_types[state.db_type_index];
                            if db == "sqlite" {
                                state.step = TuiStep::EnterDbName;
                            } else {
                                state.step = TuiStep::EnterPassword;
                            }
                        },
                        _ => {}
                    },
                    TuiStep::Confirm => match key.code {
                        KeyCode::Enter => {
                            state.step = TuiStep::Progress;
                            match tui_export_flow(&state).await {
                                Ok(msg) => {
                                    state.step = TuiStep::Done(msg);
                                }
                                Err(e) => {
                                    state.step = TuiStep::Done(format!("Export failed: {}", e));
                                }
                            }
                        }
                        KeyCode::Esc => {
                            let db = state.db_types[state.db_type_index];
                            if db == "sqlite" {
                                state.step = TuiStep::EnterDbName;
                            } else {
                                state.step = TuiStep::EnterSchema;
                            }
                        }
                        _ => {}
                    },
                    TuiStep::Done(_) => match key.code {
                        KeyCode::Esc | KeyCode::Enter => return Ok(()),
                        _ => {}
                    },
                    TuiStep::Progress => {},
                }
            }
        }
    }
}

impl TuiState {
    fn build_connection_string(&mut self) {
        let db = self.db_types[self.db_type_index];
        self.connection_string = match db {
            "sqlite" => format!("sqlite://{}", self.dbname),
            "postgres" => {
                let mut s = format!("postgres://{}
:{}@{}:{}", self.username, self.password, self.host, self.port);
                if !self.dbname.is_empty() {
                    s = format!("{}/{}", s, self.dbname);
                }
                s
            },
            "mysql" => {
                let mut s = format!("mysql://{}
:{}@{}:{}", self.username, self.password, self.host, self.port);
                if !self.dbname.is_empty() {
                    s = format!("{}/{}", s, self.dbname);
                }
                s
            },
            _ => String::new(),
        };
    }
}
