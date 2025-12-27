use notify::{
    Config, EventHandler, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind,
};
use std::path::{Component, Path, PathBuf};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// An event telling a client what to do.
///
/// This looks silly, but this is an enum to allow for future extensibility: for
/// example, a future command could reload only a single page instead of
/// everything.
#[derive(Debug, Clone)]
pub enum Event {
    Reload,
}

pub struct Watch {
    _watcher: RecommendedWatcher,
    channel: broadcast::Sender<Event>,
}

impl Watch {
    pub fn new(path: &Path) -> Self {
        let (tx, _) = broadcast::channel(16);

        let handler = Handler {
            base: std::path::absolute(path).expect("need absolute base path"),
            channel: tx.clone(),
        };
        let mut watcher = RecommendedWatcher::new(handler, Config::default()).unwrap();

        watcher.watch(path, RecursiveMode::Recursive).unwrap();

        Self {
            _watcher: watcher,
            channel: tx,
        }
    }

    pub fn stream(&self) -> BroadcastStream<Event> {
        let rx = self.channel.subscribe();
        BroadcastStream::new(rx)
    }
}

pub struct Handler {
    pub base: PathBuf,
    pub channel: broadcast::Sender<Event>,
}

impl EventHandler for Handler {
    fn handle_event(&mut self, res: notify::Result<notify::Event>) {
        if let Ok(event) = res
            && let EventKind::Modify(ModifyKind::Data(_)) = event.kind
            && !event.paths.iter().any(|p| ignore_path(&self.base, p))
        {
            // TODO debounce
            // We ignore errors when sending events: it's OK to
            // silently drop messages when there are no subscribers.
            let _ = self.channel.send(Event::Reload);
        }
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
        if let Component::Normal(name) = comp
            && crate::core::ignore_filename(name)
        {
            return true;
        }
    }
    false
}
