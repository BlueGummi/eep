use crate::*;
use crossterm::{
    cursor::Show,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::path::PathBuf;

impl Editor {
    pub fn process_command(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let cmd = self.command_buffer.trim();
        match cmd {
            "q" => {
                disable_raw_mode()?;
                execute!(self.stdout, LeaveAlternateScreen, DisableMouseCapture, Show)?;
                std::process::exit(0);
            }
            "w" => {
                if let Err(e) = self.save_file() {
                    self.set_status(&format!("Error saving file: {}", e));
                }
            }
            "wq" => {
                if let Err(e) = self.save_file() {
                    self.set_status(&format!("Error saving file: {}", e));
                } else {
                    disable_raw_mode()?;
                    execute!(self.stdout, LeaveAlternateScreen, DisableMouseCapture, Show)?;
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
        Ok(())
    }
}
