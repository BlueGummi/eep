use crate::*;
use crossterm::{
    execute,
    style::Print,
    terminal::{Clear, ClearType, EnterAlternateScreen, size},
};
use std::io::{Write, stdout};

impl Editor {
    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        execute!(stdout, Clear(ClearType::All))?;

        let (cols, rows) = size()?;
        self.screen_cols = cols as usize;
        self.screen_rows = rows as usize - 2;

        let line_num_width = if self.show_line_numbers {
            (self.content.len() as f32).log10().floor() as usize + 1
        } else {
            0
        };

        let mut output = String::new();

        if self.cursor_y >= self.offset_y + self.screen_rows - 1 {
            self.offset_y = self.cursor_y.saturating_sub(self.screen_rows - 1) + 1;
        }
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        }

        for row in 0..self.screen_rows {
            let content_row = row + self.offset_y;
            if content_row < self.content.len() {
                if self.show_line_numbers {
                    let line_num = format!(
                        "{:>width$} \x1B[90m\x1B[39m ",
                        content_row + 1,
                        width = line_num_width
                    );
                    output.push_str(&format!("\x1B[{};{}H{}", row + 1, 1, line_num));
                }

                let line = &self.content[content_row];
                let line_len = line.len();
                let start = self.offset_x;

                if start < line_len {
                    let visible_part = &line[start
                        ..std::cmp::min(start + self.screen_cols - line_num_width - 3, line_len)];
                    output.push_str(&format!(
                        "\x1B[{};{}H{}",
                        row + 1,
                        line_num_width + 3,
                        visible_part
                    ));
                }
            }
        }

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

        output.push_str(&format!(
            "\x1B[{};1H\x1B[48;5;236m\x1B[37m{}\x1B[0m",
            self.screen_rows + 1,
            &status_line[..self.screen_cols]
        ));

        let cursor_row = (self.cursor_y - self.offset_y) as u16 + 1;
        let cursor_col = if self.show_line_numbers {
            (self.cursor_x - self.offset_x) as u16 + line_num_width as u16 + 3
        } else {
            (self.cursor_x - self.offset_x) as u16 + 1
        };

        let cursor_row = std::cmp::min(cursor_row, self.screen_rows as u16);
        output.push_str(&format!("\x1B[{};{}H", cursor_row, cursor_col));

        execute!(stdout, Print(output))?;
        stdout.flush()?;
        Ok(())
    }
}
