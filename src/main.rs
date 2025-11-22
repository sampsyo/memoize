use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

/**
 * If this is a note filename, return its destination name. Otherwise, return None.
 */
fn note_dest(name: &str, dest_dir: &Utf8Path) -> Option<Utf8PathBuf> {
    if name.starts_with("_") || name.starts_with(".") {
        return None;
    }
    let (base, ext) = name.split_once(".")?;
    if ext != "md" {
        return None;
    }
    Some(dest_dir.join(format!("{}.html", base)))
}

fn render_note(src_path: &Utf8Path, dest_path: &Utf8Path) -> std::io::Result<()> {
    let source = fs::read_to_string(src_path)?;
    let parser = pulldown_cmark::Parser::new(&source);
    let out_file = fs::File::create(dest_path)?;
    pulldown_cmark::html::write_html_io(out_file, parser)?;
    Ok(())
}

fn render_all(src_dir: &Utf8Path, dest_dir: &Utf8Path) -> std::io::Result<()> {
    fs::create_dir_all(dest_dir)?;
    for entry in src_dir.read_dir_utf8()? {
        let entry = entry?;
        if let Some(dest_path) = note_dest(entry.file_name(), dest_dir) {
            render_note(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn main() {
    render_all(Utf8Path::new("."), Utf8Path::new("_public")).unwrap();
}
