pub mod assets;
pub mod markdown;

use anyhow::Result;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

assets!(TEMPLATES, "templates", ["note.html", "style.css"]);

struct Context {
    src_dir: PathBuf,
    dest_dir: PathBuf,
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

    fn render_note(&self, src_path: &Path, dest_path: &Path) -> Result<()> {
        // Render the note body.
        let source = fs::read_to_string(src_path)?;
        let (body, toc_entries) = markdown::render(&source);

        // Extract the top-level title, if any.
        let title = if let Some(first_head) = toc_entries.first()
            && first_head.level as u8 == 1
        {
            Some(first_head.title.clone())
        } else {
            None
        };

        // Get the table of contents ready for rendering.
        let toc: Vec<_> = toc_entries
            .into_iter()
            .map(|e| {
                minijinja::context! {
                    level => e.level as u8,
                    id => e.id,
                    title => e.title,
                }
            })
            .collect();

        // Render the template.
        let tmpl = self.tmpls.get_template("note.html")?;
        let out_file = fs::File::create(dest_path)?;
        tmpl.render_to_write(
            minijinja::context! {
                title => title,
                body => body,
                toc => toc,
            },
            out_file,
        )?;

        Ok(())
    }

    /// Should we skip a given file from the rendering process? We skip hidden
    /// files (prefixed with .) and ones starting with _, which are special.
    fn skip_file(name: &OsStr) -> bool {
        let bytes = name.as_encoded_bytes();
        bytes.starts_with(b".") || bytes.starts_with(b"_")
    }

    /// Given a path that is within `self.src_dir`, produce a mirrored path that
    /// is at the same place is within `self.dest_dir`.
    ///
    /// Panics if `src` is not within `self.src_dir`.
    fn mirrored_path(&self, src: &Path) -> PathBuf {
        let rel_path = src
            .strip_prefix(&self.src_dir)
            .expect("path is within root directory");
        self.dest_dir.join(rel_path)
    }

    /// If `src` is the path to a Markdown note file, return its HTML
    /// destination path. Otherwise, return None.
    fn note_dest(&self, src: &Path) -> Option<PathBuf> {
        if let Some(ext) = src.extension()
            && ext == "md"
        {
            let mut mirrored = self.mirrored_path(src);
            mirrored.set_extension("html");
            Some(mirrored)
        } else {
            None
        }
    }

    /// Given a relative path to a rendered file (i.e., something that would go
    /// in the destination directory), get the underlying resource for that
    /// path.
    fn resolve_resource(&self, rel_path: &Path) -> Option<Resource> {
        // Reject all absolute paths.
        // TODO should we also do the full path santation thing?
        if rel_path.is_absolute() {
            return None;
        }

        // If the file exists within the source directory, then this is a static
        // resource.
        let src_path = self.src_dir.join(rel_path);
        if src_path.is_file() {
            return Some(Resource::Static(src_path));
        }

        // If this is an HTML file with a corresponding note, then we'll render it.
        if let Some(ext) = rel_path.extension()
            && ext == "html"
        {
            let mut src_path = src_path;
            src_path.set_extension("md");
            if src_path.is_file() {
                return Some(Resource::Note(src_path));
            }
        }

        // Not found.
        None
    }

    fn render_site(&self) -> Result<()> {
        remove_dir_force(&self.dest_dir)?;

        // TODO parallelize rendering work
        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_entry(|e| !Self::skip_file(e.file_name()))
        {
            let entry = entry?;
            if entry.file_type().is_dir() {
                // Create mirrored directories.
                fs::create_dir_all(self.mirrored_path(entry.path()))?;
            } else if entry.file_type().is_file() {
                // Is this a Markdown note? Render it. Otherwise, just copy it.
                let src_path = entry.path();
                if let Some(dest_path) = self.note_dest(src_path) {
                    match self.render_note(src_path, &dest_path) {
                        Ok(_) => (),
                        Err(e) => {
                            eprintln!("error rendering note {}: {}", entry.path().display(), e)
                        }
                    }
                } else {
                    hard_link_or_copy(src_path, &self.mirrored_path(src_path))?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum Resource {
    Static(PathBuf),
    Note(PathBuf),
}

/// Try to hard-link `from` at `to`, falling back to a copy if the link fails
/// (e.g., the two paths are on different filesystems). This always removes the
/// current file at `to`.
fn hard_link_or_copy(from: &Path, to: &Path) -> std::io::Result<Option<u64>> {
    if to.exists() {
        fs::remove_file(to)?;
    }
    match fs::hard_link(from, to) {
        Ok(_) => Ok(None),
        Err(_) => fs::copy(from, to).map(Some),
    }
}

/// Like `std::fs::remove_dir_all`, but silently succeed if the directory already doesn't exist.
fn remove_dir_force(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Ok(()),
    }
}

fn main() {
    let ctx = Context::new(".", "_public");
    dbg!(ctx.resolve_resource(Path::new("foo.html")));
    dbg!(ctx.resolve_resource(Path::new("bar.html")));
    dbg!(ctx.resolve_resource(Path::new("stuff.txt")));
    ctx.render_site().unwrap();
}
