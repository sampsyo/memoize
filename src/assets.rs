use std::fs;
use std::path::Path;

pub trait FileList {
    fn get(&self, name: &str) -> Option<&'static str>;
    fn contents(&self) -> impl Iterator<Item = (&'static str, &'static str)>;
    fn names(&self) -> impl Iterator<Item = &'static str>;
}

type NameList = &'static [&'static str];
type ContentList = &'static [(&'static str, &'static str)];

impl FileList for NameList {
    fn get(&self, _name: &str) -> Option<&'static str> {
        None
    }

    fn contents(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        std::iter::empty()
    }

    fn names(&self) -> impl Iterator<Item = &'static str> {
        self.iter().copied()
    }
}

impl FileList for ContentList {
    fn get(&self, name: &str) -> Option<&'static str> {
        match self.iter().find(|(n, _)| *n == name) {
            Some((_, c)) => Some(c),
            None => None,
        }
    }

    fn contents(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.iter().copied()
    }

    fn names(&self) -> impl Iterator<Item = &'static str> {
        self.iter().map(|(n, _)| *n)
    }
}

pub struct Assets<F: FileList> {
    /// The directory path for this set of assets.
    dir: &'static str,

    /// The names and (possibly) contents of the assets.
    files: F,
}

impl<F: FileList> Assets<F> {
    pub fn contains(&self, name: &str) -> bool {
        self.files.names().any(|n| n == name)
    }

    pub fn load(&self, name: &str) -> std::io::Result<Option<String>> {
        if self.contains(name) {
            let path = Path::new(self.dir).join(name);
            fs::read_to_string(path).map(|c| Some(c))
        } else {
            Ok(None)
        }
    }

    pub fn get_embedded(&self, name: &str) -> Option<&'static str> {
        self.files.get(name)
    }

    pub fn embedded_files(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.files.contents()
    }
}

impl Assets<ContentList> {
    pub const fn new(dir: &'static str, contents: ContentList) -> Self {
        Self {
            dir,
            files: contents,
        }
    }
}

impl Assets<NameList> {
    pub const fn new(dir: &'static str, names: NameList) -> Self {
        Self { dir, files: names }
    }
}

pub type EmbeddedAssets = Assets<ContentList>;
pub type FileAssets = Assets<NameList>;

#[macro_export]
macro_rules! embed_assets {
    ($constname:ident, $dirname:literal, $($filename:literal),*) => {
        const $constname: $crate::assets::EmbeddedAssets = $crate::assets::EmbeddedAssets::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", $dirname),
            &[$(
                (
                    $filename,
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $dirname, "/", $filename)),
                ),
            )*],
        );
    };
}

#[macro_export]
macro_rules! file_assets {
    ($constname:ident, $dirname:literal, $($filename:literal),*) => {
        const $constname: $crate::assets::FileAssets = $crate::assets::FileAssets::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", $dirname),
            &[$( $filename, )*],
        );
    };
}

#[macro_export]
macro_rules! assets {
    ($constname:ident, $dirname:literal, $($filename:literal),*) => {
        #[cfg(debug_assertions)]
        file_assets!($constname, $dirname, "note.html");

        #[cfg(not(debug_assertions))]
        embed_assets!($constname, $dirname, "note.html");
    };
}
