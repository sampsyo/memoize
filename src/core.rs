use crate::assets::assets;
use crate::markdown;
use anyhow::Result;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::{fs, io};
use walkdir::WalkDir;

assets!(TEMPLATES, "templates", ["note.html", "style.css"]);

pub struct Context {
    src_dir: PathBuf,
    dest_dir: PathBuf,
    tmpls: minijinja::Environment<'static>,
}

impl Context {
    pub fn new(src_dir: &str, dest_dir: &str) -> Self {
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

    fn render_note_to_write<W: io::Write>(&self, src_path: &Path, dest: &mut W) -> Result<()> {
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
        tmpl.render_to_write(
            minijinja::context! {
                title => title,
                body => body,
                toc => toc,
            },
            dest,
        )?;

        Ok(())
    }

    /// Render a single Markdown note file to an HTML file.
    ///
    /// Both `src_path` and `dest_path` are complete paths to files, not
    /// relative to our source and destination directory.
    fn render_note(&self, src_path: &Path, dest_path: &Path) -> Result<()> {
        let mut out_file = fs::File::create(dest_path)?;
        self.render_note_to_write(src_path, &mut out_file)
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
    pub fn resolve_resource(&self, rel_path: &str) -> Option<Resource> {
        // Ensure that we actually have a safe, relative path fragment, and then
        // join it under the source directory.
        let rel_path = sanitize_path(rel_path)?;
        let src_path = self.src_dir.join(&rel_path);

        // If the path exists verbatim within the source directory, then this is
        // either a static file or a directory.
        if src_path.is_file() {
            return Some(Resource::Static(src_path));
        } else if src_path.is_dir() {
            return Some(Resource::Directory(src_path));
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

    pub fn render_resource<W: std::io::Write>(&self, rsrc: Resource, write: &mut W) -> Result<()> {
        match rsrc {
            Resource::Static(path) => {
                let mut file = fs::File::open(path)?;
                io::copy(&mut file, write)?;
                Ok(())
            }
            Resource::Note(path) => self.render_note_to_write(&path, write),
            Resource::Directory(path) => {
                writeln!(write, "directory: {}", path.display())?;
                Ok(())
            }
        }
    }

    pub fn render_site(&self) -> Result<()> {
        remove_dir_force(&self.dest_dir)?;

        // TODO parallelize rendering work
        for entry in WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_entry(|e| !ignore_filename(e.file_name()))
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
pub enum Resource {
    Static(PathBuf),
    Note(PathBuf),
    Directory(PathBuf),
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

/// Should we skip a given file from the rendering process? We skip hidden
/// files (prefixed with .) and ones starting with _, which are special.
fn ignore_filename(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    (bytes != b"." && bytes.starts_with(b".")) || bytes.starts_with(b"_")
}

/// Validate and relative-ize a requested path. If we return a path, it is now
/// safe to `join` with a base directory without "escaping" that directory. May
/// return `None` for any disallowed path.
fn sanitize_path(path: &str) -> Option<PathBuf> {
    let mut path_buf = PathBuf::new();
    for comp in Path::new(path).components() {
        match comp {
            Component::Normal(c) => {
                if ignore_filename(c) {
                    return None;
                } else {
                    path_buf.push(c);
                }
            }
            Component::ParentDir => return None, // Disallow `..`.
            Component::Prefix(_) => return None, // Disallow `C:`.
            Component::RootDir => (),            // Strip leading `/`.
            Component::CurDir => (),             // Ignore `.`.
        }
    }

    Some(path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute() {
        assert_eq!(sanitize_path("/hi.txt"), Some("hi.txt".into()));
    }

    #[test]
    fn relative() {
        assert_eq!(sanitize_path("hi.txt"), Some("hi.txt".into()));
    }

    #[test]
    fn with_dir() {
        assert_eq!(sanitize_path("/dir/hi.txt"), Some("dir/hi.txt".into()));
    }

    #[test]
    fn dot_dot() {
        assert_eq!(sanitize_path("/../hi.txt"), None);
    }

    #[test]
    fn dot_hidden_file() {
        assert_eq!(sanitize_path(".hi.txt"), None);
    }

    #[test]
    fn underscore_hidden_file() {
        assert_eq!(sanitize_path("_hi.txt"), None);
    }

    #[test]
    fn underscore_hidden_dir() {
        assert_eq!(sanitize_path("foo/_bar/hi.txt"), None);
    }
}
