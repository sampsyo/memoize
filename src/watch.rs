use notify::{
    Config, EventHandler, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind,
};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

const DEBOUNCE_INTERVAL: Duration = Duration::from_millis(100);

/// An event telling a client what to do.
///
/// This looks silly, but this is an enum to allow for future extensibility: for
/// example, a future command could reload only a single page instead of
/// everything.
#[derive(Debug, Clone)]
pub enum Event {
    Reload,
}

/// An active filesystem watch that emits `Event`s on changes via a Tokio
/// broadcast channel.
pub struct Watch {
    _watcher: RecommendedWatcher,
    channel: broadcast::Sender<Event>,
}

impl Watch {
    pub fn new(paths: &[&Path]) -> Self {
        let (tx, _) = broadcast::channel(16);

        let handler = Handler {
            bases: paths
                .iter()
                .map(|p| std::path::absolute(p).expect("need absolute base path"))
                .collect(),
            channel: tx.clone(),
            last_event: Instant::now(),
        };
        let mut watcher = RecommendedWatcher::new(handler, Config::default()).unwrap();
        for path in paths {
            watcher.watch(path, RecursiveMode::Recursive).unwrap();
        }

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

struct Handler {
    channel: broadcast::Sender<Event>,
    bases: Vec<PathBuf>,
    last_event: Instant,
}

impl EventHandler for Handler {
    fn handle_event(&mut self, res: notify::Result<notify::Event>) {
        // Ignore events that happen close together.
        if self.last_event.elapsed() < DEBOUNCE_INTERVAL {
            return;
        }

        // Is this a modification of a file we care about?
        if let Ok(event) = res
            && let EventKind::Modify(ModifyKind::Data(_)) = event.kind
            && !event.paths.iter().any(|p| ignore_path(&self.bases, p))
        {
            self.last_event = Instant::now();

            // We ignore errors when sending events: it's OK to
            // silently drop messages when there are no subscribers.
            let _ = self.channel.send(Event::Reload);
        }
    }
}

/// Check whether we should ignore a given path inside of base directories.
///
/// Anything outside `bases` is ignored. Inside of the base directories, any
/// file or directory with an ignored pattern is (recursively) ignored. All
/// paths must be provided in absolute form.
fn ignore_path(bases: &[PathBuf], path: &Path) -> bool {
    for base in bases {
        let frag = match path.strip_prefix(base) {
            Ok(p) => p,
            Err(_) => continue,
        };
        for comp in frag.components() {
            if let Component::Normal(name) = comp
                && crate::core::ignore_filename(name)
            {
                return true;
            }
        }
        return false;
    }
    true
}
