pub mod assets;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use walkdir::WalkDir;

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
        let body = render_markdown(&source);

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

    /// Given a path that is within `self.src_dir`, produce a mirrored path that
    /// is at the same place is within `self.dest_dir`.
    ///
    /// Panics if `src` is not within `self.src_dir`.
    fn mirrored_path(&self, src: &std::path::Path) -> std::path::PathBuf {
        let rel_path = src
            .strip_prefix(&self.src_dir)
            .expect("path is within root directory");
        self.dest_dir.join_os(rel_path)
    }

    fn render_all(&self) -> Result<()> {
        // TODO parallelize rendering work
        for entry in WalkDir::new(&self.src_dir) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                // Create mirrored directories.
                fs::create_dir_all(self.mirrored_path(entry.path()))?;
            } else if entry.file_type().is_file() {
                let file_name = entry.file_name().to_str().expect("filenames must be UTF-8");

                // Is this a Markdown note? Render it.
                if let Some(dest_path) = self.note_dest(file_name) {
                    let src_path =
                        Utf8Path::from_path(entry.path()).expect("filenames must be UTF-8");
                    match self.render_note(src_path, &dest_path) {
                        Ok(_) => (),
                        Err(e) => eprintln!("error rendering note {}: {}", file_name, e),
                    }
                }
            }
        }

        Ok(())
    }
}

fn render_markdown(source: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(&source, options);

    // TODO generate slugified anchors, produce table of contents
    // TODO gather top-level heading as title

    let mut buf = String::new();
    html::push_html(&mut buf, parser);
    buf
}

fn main() {
    let src_dir = std::env::args().into_iter().nth(1).unwrap();
    let ctx = Context::new(&src_dir, "_public");
    ctx.render_all().unwrap();
}
