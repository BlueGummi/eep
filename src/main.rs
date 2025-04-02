use eep::*;

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
