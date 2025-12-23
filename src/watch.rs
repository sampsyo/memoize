use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

pub fn blarg(path: &Path) {
    let mut watcher = RecommendedWatcher::new(
        |res| {
            match res {
                Ok(event) => eprintln!("event: {:?}", event),
                Err(error) => eprintln!("error: {}", error),
            };
            ()
        },
        Config::default(),
    )
    .unwrap();

    watcher.watch(path, RecursiveMode::Recursive).unwrap();

    loop {}
}
