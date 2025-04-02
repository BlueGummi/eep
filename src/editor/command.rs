use crate::*;
use std::io::{Write, stdout};
use std::path::PathBuf;

impl Editor {
    pub fn process_command(&mut self) {
        let cmd = self.command_buffer.trim();
        match cmd {
            "q" => {
                write!(stdout(), "\x1B[?1003l").unwrap();
                stdout().flush().unwrap();
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
                    write!(stdout(), "\x1B[?1003l").unwrap();
                    stdout().flush().unwrap();
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
}
