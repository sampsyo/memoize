use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

const TMPL_DIR: &'static str = "templates";
const NOTE_TEMPLATE: &'static str = "note.html";

struct Context {
    src_dir: Utf8PathBuf,
    dest_dir: Utf8PathBuf,
    tmpls: minijinja::Environment<'static>,
}

impl Context {
    fn new(src_dir: &str, dest_dir: &str) -> Self {
        let mut env = minijinja::Environment::new();
        env.set_loader(move |name| {
            // TODO embed in release mode
            if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
                return Ok(None);
            }
            let path = Utf8Path::new(TMPL_DIR).join(name);
            match fs::read_to_string(path) {
                Ok(source) => Ok(Some(source)),
                Err(_) => Ok(None), // TODO maybe propagate error
            }
        });

        Self {
            src_dir: src_dir.into(),
            dest_dir: dest_dir.into(),
            tmpls: env,
        }
    }

    /**
     * If this is a note filename, return its destination name. Otherwise, return None.
     */
    fn note_dest(&self, name: &str) -> Option<Utf8PathBuf> {
        if name.starts_with("_") || name.starts_with(".") {
            return None;
        }
        let (base, ext) = name.split_once(".")?;
        if ext != "md" {
            return None;
        }
        Some(self.dest_dir.join(format!("{}.html", base)))
    }

    fn render_note(&self, src_path: &Utf8Path, dest_path: &Utf8Path) -> Result<()> {
        let source = fs::read_to_string(src_path)?;
        let parser = pulldown_cmark::Parser::new(&source);
        let body = {
            let mut b = String::new();
            pulldown_cmark::html::push_html(&mut b, parser);
            b
        };

        let out_file = fs::File::create(dest_path)?;

        let tmpl = self.tmpls.get_template(NOTE_TEMPLATE)?;
        tmpl.render_to_write(
            minijinja::context! {
                body => body,
            },
            out_file,
        )?;

        Ok(())
    }

    fn render_all(&self) -> Result<()> {
        fs::create_dir_all(&self.dest_dir)?;
        // TODO parallelize
        for entry in self.src_dir.read_dir_utf8()? {
            let entry = entry?;
            if let Some(dest_path) = self.note_dest(entry.file_name()) {
                self.render_note(entry.path(), &dest_path)?;
            }
        }
        Ok(())
    }
}

fn main() {
    let ctx = Context::new(".", "_public");
    ctx.render_all().unwrap();
}
