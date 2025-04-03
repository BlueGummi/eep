use crossterm::{
    cursor::{Hide, SetCursorStyle, Show},
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, read,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::fs;
use std::io::{self, stdout};
use std::path::PathBuf;

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

pub struct Editor {
    pub content: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub mode: Mode,
    pub status_msg: String,
    pub filename: Option<PathBuf>,
    pub offset_y: usize,
    pub offset_x: usize,
    pub screen_rows: usize,
    pub screen_cols: usize,
    pub command_buffer: String,
    pub show_command: bool,
    pub tabbed: bool,
    pub show_line_numbers: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        let (cols, rows) = crossterm::terminal::size().unwrap();
        Editor {
            content: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            mode: Mode::Normal,
            status_msg: String::new(),
            filename: None,
            offset_y: 0,
            offset_x: 0,
            screen_rows: rows as usize - 2,
            screen_cols: cols as usize,
            command_buffer: String::new(),
            show_command: false,
            tabbed: false,
            show_line_numbers: true,
        }
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let content = fs::read_to_string(filename)?;
        self.content = content.lines().map(|s| s.to_string()).collect();
        if self.content.is_empty() {
            self.content.push(String::new());
        }
        self.filename = Some(PathBuf::from(filename));
        Ok(())
    }

    pub fn save_file(&mut self) -> io::Result<()> {
        if let Some(ref filename) = self.filename {
            let mut content = self.content.join("\n");
            if content.chars().last().is_some_and(|c| c != '\n') {
                content.push('\n');
            }
            fs::write(filename, content)?;
            self.status_msg = format!("Saved '{}'", filename.display());
        } else {
            self.set_status("No filename specified. Use :w <filename>");
        }
        Ok(())
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_msg = msg.to_string();
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = stdout();

        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture,)?;
        execute!(stdout, SetCursorStyle::SteadyBlock)?;

        self.render()?;

        loop {
            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => break,

                Event::Key(KeyEvent {
                    code: KeyCode::Char(':'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.mode = Mode::Command;
                    self.show_command = true;
                    self.command_buffer.clear();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('i'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    execute!(stdout, SetCursorStyle::SteadyUnderScore)?;
                    self.mode = Mode::Insert;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    execute!(stdout, SetCursorStyle::SteadyBlock)?;
                    if self.mode == Mode::Command {
                        self.command_buffer.clear();
                        self.show_command = false;
                    }
                    self.mode = Mode::Normal;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Insert => {
                    self.delete_char();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Command => {
                    self.command_buffer.pop();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Insert => {
                    self.insert_newline();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Command => {
                    self.process_command()?;
                    self.mode = Mode::Normal;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Insert => {
                    self.insert_char(c);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Command => {
                    self.command_buffer.push(c);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.move_cursor(KeyCode::Left);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.move_cursor(KeyCode::Down);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.move_cursor(KeyCode::Up);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('l'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.move_cursor(KeyCode::Right);
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('0'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.cursor_x = 0;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('$'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.cursor_x = self.content[self.cursor_y].len();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('G'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.cursor_y = self.content.len() - 1;
                    self.cursor_x = self.content[self.cursor_y].len();
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('g'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.cursor_y = 0;
                    self.cursor_x = 0;
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('x'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    if self.cursor_x < self.content[self.cursor_y].len() {
                        self.content[self.cursor_y].remove(self.cursor_x);
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    if self.content.len() > 1 {
                        self.content.remove(self.cursor_y);
                        if self.cursor_y >= self.content.len() {
                            self.cursor_y = self.content.len() - 1;
                        }
                        self.cursor_x = 0;
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('u'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.set_status("Undo not implemented yet");
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Char('/'),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if self.mode == Mode::Normal => {
                    self.set_status("Search not implemented yet");
                }

                Event::Key(key_event) => self.move_cursor(key_event.code),
                Event::Mouse(event) => self.handle_mouse_event(event),
                _ => {}
            }

            self.scroll();
            self.render()?;
        }
        disable_raw_mode()?;
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture, Show)?;
        Ok(())
    }
}
