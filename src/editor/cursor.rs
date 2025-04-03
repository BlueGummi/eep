use crate::*;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};

impl Editor {
    pub fn move_cursor(&mut self, direction: KeyCode) {
        match direction {
            KeyCode::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_y < self.content.len() - 1 {
                    self.cursor_y += 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.content[self.cursor_y].len();
                }
            }
            KeyCode::Right => {
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

    pub fn scroll(&mut self) {
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

    pub fn insert_char(&mut self, c: char) {
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

    pub fn delete_char(&mut self) {
        if self.cursor_x == 0 && self.cursor_y == 0 {
            return;
        }
        if self.cursor_x > 0 {
            self.content[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            let current_line = self.content.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.content[self.cursor_y].len();
            self.content[self.cursor_y].push_str(&current_line);
        }
    }

    pub fn insert_newline(&mut self) {
        let current_line = self.content[self.cursor_y].split_off(self.cursor_x);
        self.content.insert(self.cursor_y + 1, current_line);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::ScrollUp => {
                self.move_cursor(KeyCode::Up);
                self.move_cursor(KeyCode::Up);
                self.move_cursor(KeyCode::Up);
            }
            MouseEventKind::ScrollDown => {
                self.move_cursor(KeyCode::Down);
                self.move_cursor(KeyCode::Down);
                self.move_cursor(KeyCode::Down);
            }
            _ => (),
        }
    }
}
