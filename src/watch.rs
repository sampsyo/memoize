use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};
use std::path::Path;
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
        let channel = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res
                    && let EventKind::Modify(ModifyKind::Data(_)) = event.kind
                {
                    // TODO check for at least one non-ignored path in `event.paths`
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
