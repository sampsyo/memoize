use crate::assets::assets;
use crate::{git, markdown, parallel};
use anyhow::Result;
use serde::Deserialize;
use std::ffi::OsStr;
use std::num::NonZero;
use std::path::{Component, Path, PathBuf};
use std::{fs, io};
use walkdir::WalkDir;

assets!(
    TEMPLATES,
    "templates",
    ["note.html", "style.css", "livereload.js"]
);

pub struct Context {
    pub src_dir: PathBuf,
    pub livereload: bool,
    pub config: Config,
    tmpls: minijinja::Environment<'static>,
}

impl Context {
    pub fn new(src_dir: &str, livereload: bool, config: Config) -> Self {
        let mut ctx = Self {
            src_dir: src_dir.into(),
            tmpls: minijinja::Environment::new(),
            livereload,
            config,
        };

        // Register embedded templates, which are available in release mode.
        #[cfg(not(debug_assertions))]
        for (name, source) in TEMPLATES.contents() {
            ctx.tmpls
                .add_template(name, source)
                .expect("error in embedded template");
        }

        // In debug mode only, load templates directly from the filesystem.
        #[cfg(debug_assertions)]
        ctx.reload_templates();

        ctx
    }

    /// Re-read all templates from the filesystem.
    pub fn reload_templates(&mut self) {
        self.tmpls.clear_templates();
        for (name, source) in TEMPLATES.read_all() {
            self.tmpls
                .add_template_owned(name, source.expect("error reading template"))
                .expect("error in loaded template");
        }
    }

    /// Render the HTML page for a given Markdown note.
    pub fn render_note<W: io::Write>(&self, src_path: &Path, dest: &mut W) -> Result<()> {
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

        // Get git commit info.
        let commit = git::last_commit(&self.src_dir, src_path).map(|c| {
            let info = c.info();
            minijinja::context! {
                hash => info.hash,
                short_hash => info.hash[..7],
                date => info.date,
                email => info.email,
                name => info.name,
            }
        });

        // Filename info.
        let rel_path = src_path
            .strip_prefix(&self.src_dir)
            .expect("note path must be within source directory")
            .to_string_lossy();
        let file_name = src_path.file_name().expect("no filename").to_string_lossy();
        let edit_link = self
            .config
            .edit_link_prefix
            .as_ref()
            .map(|p| format!("{p}{rel_path}"));

        // Render the template.
        let tmpl = self.tmpls.get_template("note.html")?;
        tmpl.render_to_write(
            minijinja::context! {
                title => title,
                body => body,
                toc => toc,
                livereload => self.livereload,
                git => commit,
                path => rel_path,
                name => file_name,
                edit_link => edit_link,
            },
            dest,
        )?;

        Ok(())
    }

    /// Render a single Markdown note file to an HTML file.
    ///
    /// Both `src_path` and `dest_path` are complete paths to files, not
    /// relative to our source and destination directory.
    fn render_note_to_file(&self, src_path: &Path, dest_path: &Path) -> Result<()> {
        let mut out_file = fs::File::create(dest_path)?;
        self.render_note(src_path, &mut out_file)
    }

    /// Render any resource.
    pub fn render_resource<W: std::io::Write>(&self, rsrc: Resource, dest: &mut W) -> Result<()> {
        match rsrc {
            Resource::Static(path) => {
                let mut file = fs::File::open(path)?;
                io::copy(&mut file, dest)?;
                Ok(())
            }
            Resource::Note(path) => self.render_note(&path, dest),
            Resource::Directory(path) => {
                // TODO this is where we'd generate index pages
                writeln!(dest, "directory: {}", path.display())?;
                Ok(())
            }
        }
    }

    /// Given a path that is within `self.src_dir`, produce a mirrored path that
    /// is at the same place is within `dest_dir`.
    ///
    /// Panics if `src` is not within `self.src_dir`.
    fn dest_path(&self, src: &Path, dest_dir: &Path) -> PathBuf {
        let rel_path = src
            .strip_prefix(&self.src_dir)
            .expect("path is within root directory");
        dest_dir.join(rel_path)
    }

    /// Assuming `src` is the path to a Markdown note file, return its HTML
    /// destination path.
    ///
    /// Panics if `src` is not a note file within `self.src_dir`.
    fn note_dest_path(&self, src: &Path, dest_dir: &Path) -> PathBuf {
        assert!(is_note(src), "must be a note path");
        let mut mirrored = self.dest_path(src, dest_dir);
        mirrored.set_extension("html");
        mirrored
    }

    /// Given a relative path to a rendered file (i.e., something that would go
    /// in the destination directory), look up the underlying resource for that
    /// path, if one exists.
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

    /// List all the resources in the source directory.
    pub fn read_resources(&self) -> impl Iterator<Item = Resource> {
        WalkDir::new(&self.src_dir)
            .into_iter()
            .filter_entry(|e| !ignore_filename(e.file_name()))
            .filter_map(|entry| match entry {
                Ok(entry) => {
                    if entry.file_type().is_dir() {
                        Some(Resource::Directory(entry.path().into()))
                    } else if entry.file_type().is_file() {
                        if is_note(entry.path()) {
                            Some(Resource::Note(entry.path().into()))
                        } else {
                            Some(Resource::Static(entry.path().into()))
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("directory walk error: {}", e);
                    None
                }
            })
    }

    /// Render all resources in a site to a destination directory.
    pub fn render_site(&self, threads: Option<NonZero<usize>>, dest_dir: &Path) -> Result<()> {
        parallel::scope_with_threads(threads, |pool| {
            remove_dir_force(dest_dir)?;

            for rsrc in self.read_resources() {
                match rsrc {
                    Resource::Directory(src_path) => {
                        fs::create_dir_all(self.dest_path(&src_path, dest_dir))?;
                    }
                    Resource::Static(src_path) => {
                        hard_link_or_copy(&src_path, &self.dest_path(&src_path, dest_dir))?;
                    }
                    Resource::Note(src_path) => {
                        pool.spawn(move || {
                            let dest_path = self.note_dest_path(&src_path, dest_dir);
                            match self.render_note_to_file(&src_path, &dest_path) {
                                Ok(_) => (),
                                Err(e) => {
                                    eprintln!("error rendering note {}: {}", src_path.display(), e)
                                }
                            }
                        });
                    }
                }
            }

            Ok(())
        })
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
pub fn ignore_filename(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    (bytes != b"." && bytes.starts_with(b".")) || bytes.starts_with(b"_")
}

/// Does this source filename look like a Markdown note file?
fn is_note(path: &Path) -> bool {
    matches!(path.extension(), Some(e) if e == "md")
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

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    edit_link_prefix: Option<String>,
}

impl Config {
    pub fn load(src_dir: &Path) -> Result<Self> {
        match fs::read_to_string(src_dir.join("_config.toml")) {
            // Silently proceed if the file isn't found, but crash on other errors.
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e)?,
            Ok(s) => Ok(toml::from_str(&s)?),
        }
    }
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
