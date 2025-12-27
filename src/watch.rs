use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};
use std::path::{Component, Path};
use tokio::sync::broadcast;

pub struct Watch {
    pub watcher: RecommendedWatcher,
    pub channel: broadcast::Sender<Event>,
}

// TODO this looks silly, but we have an enum here for possible future
// extensibility (only reloading one page instead of all of them)
#[derive(Debug, Clone)]
pub enum Event {
    Reload,
}

impl Watch {
    pub fn new(path: &Path) -> Self {
        let (tx, _) = broadcast::channel(16);

        // TODO are these clones really necessary?
        let channel = tx.clone();
        let base = std::path::absolute(path).expect("need absolute base path");
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res
                    && let EventKind::Modify(ModifyKind::Data(_)) = event.kind
                    && !event.paths.iter().any(|p| ignore_path(&base, p))
                {
                    // TODO debounce
                    // We ignore errors when sending events: it's OK to
                    // silently drop messages when there are no subscribers.
                    let _ = tx.send(Event::Reload);
                }
            },
            Config::default(),
        )
        .unwrap();

        watcher.watch(path, RecursiveMode::Recursive).unwrap();

        Self { watcher, channel }
    }
}

/// Check whether we should ignore a given path inside of a base directory.
///
/// It's ignored if any component below `base` is an ignored filename. Also,
/// anything outside `base` is also ignored. Both arguments must be provided as
/// absolute paths.
fn ignore_path(base: &Path, path: &Path) -> bool {
    let frag = match path.strip_prefix(base) {
        Ok(p) => p,
        Err(_) => return true,
    };
    for comp in frag.components() {
        if let Component::Normal(name) = comp {
            if crate::core::ignore_filename(name) {
                return true;
            }
        }
    }
    return false;
}
