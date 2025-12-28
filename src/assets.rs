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
    pub dir: &'static str,

    /// The names and (possibly) contents of the assets.
    files: F,
}

impl<F: FileList> Assets<F> {
    /// Check whether a given asset file is available (embedded or on disk).
    pub fn contains(&self, name: &str) -> bool {
        self.files.names().any(|n| n == name)
    }

    /// Read an asset file from disk.
    pub fn read(&self, name: &str) -> std::io::Result<Option<String>> {
        if self.contains(name) {
            let path = Path::new(self.dir).join(name);
            fs::read_to_string(path).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Read all assets from disk, returning their name and contents.
    pub fn read_all(&self) -> impl Iterator<Item = (&'static str, std::io::Result<String>)> {
        self.files.names().map(|name| match self.read(name) {
            Ok(c) => (name, Ok(c.expect("registered file not found"))),
            Err(e) => (name, Err(e)),
        })
    }

    /// Get the embedded contents of a file. If this is a filesystem-only asset
    /// set, this always returns None.
    pub fn get(&self, name: &str) -> Option<&'static str> {
        self.files.get(name)
    }

    /// Get all the embedded files, iterating over `(name, contents)` pairs. If
    /// this is a filesystem-only asset set, this is always empty.
    pub fn contents(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
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

/// Embed a list of asset files in the binary.
#[macro_export]
macro_rules! embed_assets {
    ($constname:ident, $dirname:literal, [ $($filename:literal),* ]) => {
        pub(crate) const $constname: $crate::assets::EmbeddedAssets = $crate::assets::EmbeddedAssets::new(
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

/// Provide access to a list of asset files in the filesystem.
#[macro_export]
macro_rules! file_assets {
    ($constname:ident, $dirname:literal, [ $($filename:literal),* ]) => {
        pub(crate) const $constname: $crate::assets::FileAssets = $crate::assets::FileAssets::new(
            concat!(env!("CARGO_MANIFEST_DIR"), "/", $dirname),
            &[$( $filename, )*],
        );
    };
}

/// Either embed asset files or use them directly from the filesystem, depending
/// on whether we're building in debug or release mode.
#[macro_export]
macro_rules! assets {
    ($constname:ident, $dirname:literal, [ $($filename:literal),* ]) => {
        #[cfg(debug_assertions)]
        $crate::assets::file_assets!($constname, $dirname, [ $($filename),* ]);

        #[cfg(not(debug_assertions))]
        $crate::assets::embed_assets!($constname, $dirname, [ $($filename),* ]);
    };
}

pub(crate) use assets;

#[allow(unused_imports)]
pub(crate) use embed_assets;

#[allow(unused_imports)]
pub(crate) use file_assets;
