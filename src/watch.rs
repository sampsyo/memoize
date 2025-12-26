use crate::core::Context;
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
                match res {
                    Ok(event) => {
                        eprintln!("event: {:?}", event);
                        if let EventKind::Modify(ModifyKind::Data(_)) = event.kind {
                            for path in event.paths.iter() {
                                // TODO check if it's ignored
                                dbg!(path);
                            }
                        }
                        match tx.send(Event::Reload) {
                            Ok(_) => (),
                            Err(e) => eprintln!("channel send error: {e}"),
                        }
                    }
                    Err(error) => eprintln!("error: {}", error),
                };
            },
            Config::default(),
        )
        .unwrap();

        watcher.watch(&path, RecursiveMode::Recursive).unwrap();

        Self { watcher, channel }
    }
}

#[tokio::main]
pub async fn blarg(ctx: Context) {
    let watch = Watch::new(&ctx.src_dir);

    let mut rx = watch.channel.subscribe();
    tokio::spawn(async move {
        loop {
            let event = rx.recv().await.unwrap();
            dbg!(event);
        }
    })
    .await
    .unwrap();
}
