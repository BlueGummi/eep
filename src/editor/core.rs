use std::fs;
use std::io::{self, Write, stdout};
use std::path::PathBuf;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{
    cursor::{self},
    terminal_size,
};

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
        let (cols, rows) = terminal_size().unwrap();
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
        let stdin = io::stdin();

        let mut stdout = stdout().into_raw_mode().unwrap();

        write!(stdout, "\x1B[?1003h").unwrap();
        stdout.flush().unwrap();

        self.render();

        for event in stdin.events() {
            match event? {
                Event::Key(Key::Char('q')) if self.mode == Mode::Normal => break,
                Event::Key(Key::Char(':')) if self.mode == Mode::Normal => {
                    self.mode = Mode::Command;
                    self.show_command = true;
                    self.command_buffer.clear();
                }
                Event::Key(Key::Char('i')) if self.mode == Mode::Normal => {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    write!(handle, "{}", cursor::SteadyBar)?;
                    handle.flush()?;
                    self.mode = Mode::Insert;
                }
                Event::Key(Key::Esc) => {
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    write!(handle, "{}", cursor::SteadyBlock)?;
                    handle.flush()?;
                    if self.mode == Mode::Command {
                        self.command_buffer.clear();
                        self.show_command = false;
                    }
                    self.mode = Mode::Normal;
                }
                Event::Key(Key::Backspace) if self.mode == Mode::Insert => {
                    self.delete_char();
                }
                Event::Key(Key::Backspace) if self.mode == Mode::Command => {
                    self.command_buffer.pop();
                }
                Event::Key(Key::Char('\n')) if self.mode == Mode::Insert => {
                    self.insert_newline();
                }
                Event::Key(Key::Char('\n')) if self.mode == Mode::Command => {
                    self.process_command();
                    self.mode = Mode::Normal;
                }
                Event::Key(Key::Char('n')) if self.mode == Mode::Normal => {
                    self.show_line_numbers = !self.show_line_numbers;
                }
                Event::Key(Key::Char(c)) if self.mode == Mode::Insert => {
                    self.insert_char(c);
                }
                Event::Key(Key::Char(c)) if self.mode == Mode::Command => {
                    self.command_buffer.push(c);
                }
                Event::Key(Key::Char('h')) if self.mode == Mode::Normal => {
                    self.move_cursor(Key::Left)
                }
                Event::Key(Key::Char('j')) if self.mode == Mode::Normal => {
                    self.move_cursor(Key::Down)
                }
                Event::Key(Key::Char('k')) if self.mode == Mode::Normal => {
                    self.move_cursor(Key::Up)
                }
                Event::Key(Key::Char('l')) if self.mode == Mode::Normal => {
                    self.move_cursor(Key::Right)
                }
                Event::Key(Key::Char('0')) if self.mode == Mode::Normal => self.cursor_x = 0,
                Event::Key(Key::Char('$')) if self.mode == Mode::Normal => {
                    self.cursor_x = self.content[self.cursor_y].len()
                }
                Event::Key(Key::Char('G')) if self.mode == Mode::Normal => {
                    self.cursor_y = self.content.len() - 1;
                    self.cursor_x = self.content[self.cursor_y].len();
                }
                Event::Key(Key::Char('g')) if self.mode == Mode::Normal => {
                    self.cursor_y = 0;
                    self.cursor_x = 0;
                }
                Event::Key(Key::Char('x')) if self.mode == Mode::Normal => {
                    if self.cursor_x < self.content[self.cursor_y].len() {
                        self.content[self.cursor_y].remove(self.cursor_x);
                    }
                }
                Event::Key(Key::Char('d')) if self.mode == Mode::Normal => {
                    if self.content.len() > 1 {
                        self.content.remove(self.cursor_y);
                        if self.cursor_y >= self.content.len() {
                            self.cursor_y = self.content.len() - 1;
                        }
                        self.cursor_x = 0;
                    }
                }
                Event::Key(Key::Char('u')) if self.mode == Mode::Normal => {
                    self.set_status("Undo not implemented yet");
                }
                Event::Key(Key::Char('/')) if self.mode == Mode::Normal => {
                    self.set_status("Search not implemented yet");
                }
                Event::Key(key) => self.move_cursor(key),
                Event::Mouse(me) => self.handle_mouse_event(me),
                _ => {}
            }

            self.scroll();
            self.render();
        }
        write!(stdout, "\x1B[?1003l").unwrap();
        stdout.flush().unwrap();
        Ok(())
    }
}
