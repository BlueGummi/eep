use std::fs;
use std::io::{self, stdout, Write};
use std::path::PathBuf;
use termion::color;
use termion::event::{Event, Key, MouseButton, MouseEvent};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{
    clear,
    cursor::{self},
    screen::IntoAlternateScreen,
    terminal_size,
};

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

struct Editor {
    content: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    mode: Mode,
    status_msg: String,
    filename: Option<PathBuf>,
    offset_y: usize,
    offset_x: usize,
    screen_rows: usize,
    screen_cols: usize,
    command_buffer: String,
    show_command: bool,
    tabbed: bool,
    show_line_numbers: bool,
}

impl Editor {
    fn new() -> Self {
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

    fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let content = fs::read_to_string(filename)?;
        self.content = content.lines().map(|s| s.to_string()).collect();
        if self.content.is_empty() {
            self.content.push(String::new());
        }
        self.filename = Some(PathBuf::from(filename));
        Ok(())
    }

    fn save_file(&mut self) -> io::Result<()> {
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

    fn set_status(&mut self, msg: &str) {
        self.status_msg = msg.to_string();
    }

    fn move_cursor(&mut self, direction: Key) {
        match direction {
            Key::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                }
            }
            Key::Down => {
                if self.cursor_y < self.content.len() - 1 {
                    self.cursor_y += 1;
                }
            }
            Key::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.content[self.cursor_y].len();
                }
            }
            Key::Right => {
                if self.cursor_x < self.content[self.cursor_y].len() {
                    self.cursor_x += 1;
                } else if self.cursor_y < self.content.len() - 1 {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
            }
            _ => {}
        }

        let line_len = self.content[self.cursor_y].len();
        if self.cursor_x > line_len {
            self.cursor_x = line_len;
        }
    }

    fn scroll(&mut self) {
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + self.screen_rows {
            self.offset_y = self.cursor_y - self.screen_rows + 1;
        }

        if self.cursor_x < self.offset_x {
            self.offset_x = self.cursor_x;
        } else if self.cursor_x >= self.offset_x + self.screen_cols {
            self.offset_x = self.cursor_x - self.screen_cols + 1;
        }
    }

    fn insert_char(&mut self, c: char) {
        if self.cursor_y >= self.content.len() {
            self.content.push(String::new());
        }
        if c == '\t' {
            for _ in 0..4 {
                self.content[self.cursor_y].insert(self.cursor_x, ' ');
            }
            self.tabbed = true;
            self.cursor_x += 4;
            return;
        } else {
            self.tabbed = false;
        }
        self.content[self.cursor_y].insert(self.cursor_x, c);
        self.cursor_x += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_x == 0 && self.cursor_y == 0 {
            return;
        }
        if self.cursor_x > 0 {
            let mut remove_four = true;
            for i in 1..=4 {
                if self.content[self.cursor_y]
                    .char_indices()
                    .nth(self.cursor_x - i)
                    .is_some_and(|(_, value)| value != ' ')
                {
                    remove_four = false;
                }
            }
            self.content[self.cursor_y].remove(self.cursor_x - 1);
            if remove_four {
                let length = if self.content[self.cursor_y].len() > 3 {
                    4
                } else {
                    self.content[self.cursor_y].len()
                };
                for i in 2..=length {
                    self.content[self.cursor_y].remove(self.cursor_x - i);
                }
                self.cursor_x -= 3;
            }
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            let current_line = self.content.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.content[self.cursor_y].len();
            self.content[self.cursor_y].push_str(&current_line);
        }
    }

    fn insert_newline(&mut self) {
        let current_line = self.content[self.cursor_y].split_off(self.cursor_x);
        self.content.insert(self.cursor_y + 1, current_line);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    fn process_command(&mut self) {
        let cmd = self.command_buffer.trim();
        match cmd {
            "q" => std::process::exit(0),
            "w" => {
                if let Err(e) = self.save_file() {
                    self.set_status(&format!("Error saving file: {}", e));
                }
            }
            "wq" => {
                if let Err(e) = self.save_file() {
                    self.set_status(&format!("Error saving file: {}", e));
                } else {
                    std::process::exit(0);
                }
            }
            _ if cmd.starts_with("w ") => {
                let filename = cmd[2..].trim();
                self.filename = Some(PathBuf::from(filename));
                if let Err(e) = self.save_file() {
                    self.set_status(&format!("Error saving file: {}", e));
                }
            }
            _ => self.set_status(&format!("Unknown command: {}", cmd)),
        }
        self.command_buffer.clear();
        self.show_command = false;
    }

    fn jump_to_line(&mut self, line_num: usize) {
        if line_num > 0 && line_num <= self.content.len() {
            self.cursor_y = line_num - 1;
            self.cursor_x = 0;
        }
    }

    fn render(&mut self) {
        let mut screen = std::io::stdout()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap();
        write!(screen, "{}", clear::All).unwrap();

        let (cols, rows) = termion::terminal_size().unwrap();
        self.screen_cols = cols as usize;
        self.screen_rows = rows as usize - 2;

        let line_num_width = if self.show_line_numbers {
            (self.content.len() as f32).log10().floor() as usize + 1
        } else {
            0
        };

        for row in 0..self.screen_rows {
            let content_row = row + self.offset_y;
            if content_row < self.content.len() {
                if self.show_line_numbers {
                    let line_num = format!("{:>width$} ", content_row + 1, width = line_num_width);
                    write!(
                        screen,
                        "{}{}{}",
                        cursor::Goto(1, (row + 1) as u16),
                        color::Fg(color::LightBlack),
                        line_num
                    )
                    .unwrap();
                }

                let line = &self.content[content_row];
                let line_len = line.len();
                let start = self.offset_x;
                let end = std::cmp::min(start + self.screen_cols - line_num_width, line_len);

                if start < line_len {
                    let visible_part = &line[start..end];
                    write!(
                        screen,
                        "{}{}{}",
                        cursor::Goto((line_num_width + 2) as u16, (row + 1) as u16),
                        color::Fg(color::Reset),
                        visible_part
                    )
                    .unwrap();
                }
            }
        }

        write!(screen, "{}", cursor::Goto(1, (self.screen_rows + 1) as u16)).unwrap();
        write!(
            screen,
            "{}{}",
            color::Bg(color::LightBlack),
            color::Fg(color::White)
        )
        .unwrap();

        let mode = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        };

        let filename = self
            .filename
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "[No Name]".to_string());

        let status_left = format!("{} -- {} -- ", filename, mode);
        let status_right = format!(
            "Ln {}/{} Col {}",
            self.cursor_y + 1,
            self.content.len(),
            self.cursor_x + 1
        );

        let status_msg = if !self.status_msg.is_empty() {
            &self.status_msg
        } else if self.show_command {
            &format!(":{}", self.command_buffer)
        } else {
            ""
        };

        let available_space = self
            .screen_cols
            .saturating_sub(status_left.len() + status_right.len());
        let status_middle = if status_msg.len() > available_space {
            &status_msg[..available_space]
        } else {
            status_msg
        };

        let padding = available_space.saturating_sub(status_middle.len());
        let status_line = format!(
            "{}{}{}{}{}",
            status_left,
            status_middle,
            " ".repeat(padding),
            status_right,
            " ".repeat(self.screen_cols.saturating_sub(
                status_left.len() + status_middle.len() + padding + status_right.len()
            ))
        );

        write!(screen, "{}", &status_line[..self.screen_cols]).unwrap();
        write!(
            screen,
            "{}{}",
            color::Bg(color::Reset),
            color::Fg(color::Reset)
        )
        .unwrap();

        let cursor_row = (self.cursor_y - self.offset_y) as u16 + 1;
        let cursor_col = if self.show_line_numbers {
            (self.cursor_x - self.offset_x) as u16 + line_num_width as u16 + 2
        } else {
            (self.cursor_x - self.offset_x) as u16 + 1
        };
        write!(screen, "{}", cursor::Goto(cursor_col, cursor_row)).unwrap();
        screen.flush().unwrap();
    }
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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

    fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Press(button, _, _) => match button {
                MouseButton::WheelUp => {
                    self.move_cursor(Key::Up);
                    self.move_cursor(Key::Up);
                    self.move_cursor(Key::Up);
                }
                MouseButton::WheelDown => {
                    self.move_cursor(Key::Down);
                    self.move_cursor(Key::Down);
                    self.move_cursor(Key::Down);
                }
                _ => (),
            },
            MouseEvent::Release(_, _) => {}
            MouseEvent::Hold(_, _) => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut editor = Editor::new();

    if let Some(filename) = std::env::args().nth(1) {
        if let Err(e) = editor.open_file(&filename) {
            eprintln!("Failed to open {}: {}", filename, e);
            std::process::exit(1);
        }
    }

    editor.run()?;
    Ok(())
}
