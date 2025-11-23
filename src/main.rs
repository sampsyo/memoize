pub mod assets;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

assets!(TEMPLATES, "templates", ["note.html"]);

struct Context {
    src_dir: Utf8PathBuf,
    dest_dir: Utf8PathBuf,
    tmpls: minijinja::Environment<'static>,
}

impl Context {
    fn new(src_dir: &str, dest_dir: &str) -> Self {
        let mut env = minijinja::Environment::new();

        // Register embedded templates, which are available in release mode.
        for (name, source) in TEMPLATES.contents() {
            env.add_template(name, source)
                .expect("embedded template must be valid Jinja code");
        }

        // In debug mode only, load templates directly from the filesystem.
        #[cfg(debug_assertions)]
        env.set_loader(|name| {
            match TEMPLATES.read(name) {
                Ok(source) => Ok(source),
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
        Some(self.dest_dir.join(format!("{base}.html")))
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

        let tmpl = self.tmpls.get_template("note.html")?;
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
                match self.render_note(entry.path(), &dest_path) {
                    Ok(_) => (),
                    Err(e) => eprintln!("error rendering note {}: {}", entry.file_name(), e),
                }
            }
        }
        Ok(())
    }
}

fn main() {
    let ctx = Context::new(".", "_public");
    ctx.render_all().unwrap();
}
