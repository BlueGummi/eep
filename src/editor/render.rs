use crate::*;
use std::io::Write;
use termion::color;
use termion::raw::IntoRawMode;
use termion::{
    clear,
    cursor::{self},
    screen::IntoAlternateScreen,
};
impl Editor {
    pub fn render(&mut self) {
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
}
