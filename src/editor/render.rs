use crate::*;
use crossterm::cursor::SetCursorStyle;
use crossterm::{
    execute,
    style::Print,
    terminal::{size, Clear, ClearType, EnterAlternateScreen},
};
use std::io::Write;

const STATUS_FILENAME_FG: &str = "\x1B[38;5;231m"; // White text
const STATUS_FILENAME_BG: &str = "\x1B[48;5;240m"; // Dark gray background
const STATUS_MODE_FG: &str = "\x1B[35;5;213m";
const STATUS_MODE_BG: &str = "\x1B[48;5;236m"; // Darker gray background
const STATUS_MSG_FG: &str = "\x1B[38;5;220m"; // Yellow text
const STATUS_MSG_BG: &str = "\x1B[48;5;236m"; // Darker gray background
const STATUS_CMD_FG: &str = "\x1B[38;5;117m"; // Light blue text
const STATUS_CMD_BG: &str = "\x1B[48;5;236m"; // Darker gray background
const STATUS_INFO_FG: &str = "\x1B[38;5;255m"; // Light gray text
const STATUS_INFO_BG: &str = "\x1B[48;5;236m"; // Darker gray background
const STATUS_TRANSPARENT_BG: &str = "\x1B[49m"; // Transparent background
const RESET: &str = "\x1B[0m";

impl Editor {
    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(self.stdout, EnterAlternateScreen)?;
        execute!(self.stdout, Clear(ClearType::All))?;

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
                        ../*std::cmp::min(start + self.screen_cols - line_num_width - 3, */line_len];//)];
                    output.push_str(&format!(
                        "\x1B[{};{}H{}",
                        row + 1,
                        line_num_width + 3,
                        visible_part
                    ));
                }
            }
        }

        let status_bar = self.build_status_bar();
        output.push_str(&status_bar);

        let cursor_row = (self.cursor_y - self.offset_y) as u16 + 1;
        let cursor_col = if self.show_line_numbers {
            (self.cursor_x - self.offset_x) as u16 + line_num_width as u16 + 3
        } else {
            (self.cursor_x - self.offset_x) as u16 + 1
        };
        match self.mode {
            Mode::Normal => execute!(self.stdout, SetCursorStyle::SteadyBlock)?,
            Mode::Insert => execute!(self.stdout, SetCursorStyle::SteadyBar)?,
            Mode::Command => execute!(self.stdout, SetCursorStyle::SteadyBlock)?,
        }
        let cursor_row = std::cmp::min(cursor_row, self.screen_rows as u16);
        output.push_str(&format!("\x1B[{};{}H", cursor_row, cursor_col));

        execute!(self.stdout, Print(output))?;
        self.stdout.flush()?;
        Ok(())
    }

    fn build_status_bar(&self) -> String {
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

        let left_segment = format!(
            "{}{}{} -- {}{}{} -- ",
            STATUS_FILENAME_FG, filename, RESET, STATUS_MODE_FG, mode, RESET,
        );

        let right_segment = format!(
            "{}{}Ln {}/{} Col {}",
            STATUS_INFO_FG,
            STATUS_TRANSPARENT_BG,
            self.cursor_y + 1,
            self.content.len(),
            self.cursor_x + 1
        );

        let middle_content = if !self.status_msg.is_empty() {
            format!(
                "{}{}{}",
                STATUS_MSG_FG, STATUS_TRANSPARENT_BG, self.status_msg
            )
        } else if self.show_command {
            format!(
                "{}{}:{}",
                STATUS_CMD_FG, STATUS_TRANSPARENT_BG, self.command_buffer
            )
        } else {
            String::new()
        };

        let left_len = visible_length(&left_segment);
        let right_len = visible_length(&right_segment);
        let available_space = self.screen_cols.saturating_sub(left_len + right_len);

        let middle_segment = if visible_length(&middle_content) > available_space {
            let truncated = truncate_visible(&middle_content, available_space);
            format!(
                "{}{}",
                truncated,
                " ".repeat(available_space.saturating_sub(visible_length(&truncated)))
            )
        } else {
            format!(
                "{}{}",
                middle_content,
                " ".repeat(available_space.saturating_sub(visible_length(&middle_content)))
            )
        };

        format!(
            "\x1B[{};1H{}{}{}{}{}",
            self.screen_rows + 1,
            left_segment,
            middle_segment,
            right_segment,
            " ".repeat(
                self.screen_cols
                    .saturating_sub(left_len + visible_length(&middle_segment) + right_len)
            ),
            RESET
        )
    }
}

fn visible_length(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1B' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }

    len
}

fn truncate_visible(s: &str, max_len: usize) -> String {
    let mut result = String::new();
    let mut current_len = 0;
    let mut in_escape = false;
    let mut current_escape = String::new();

    for c in s.chars() {
        if current_len >= max_len {
            break;
        }

        if c == '\x1B' {
            in_escape = true;
            current_escape.push(c);
        } else if in_escape {
            current_escape.push(c);
            if c == 'm' {
                result.push_str(&current_escape);
                current_escape.clear();
                in_escape = false;
            }
        } else {
            result.push(c);
            current_len += 1;
        }
    }

    if current_len >= max_len && !result.ends_with(RESET) {
        result.push_str(RESET);
    }

    result
}
