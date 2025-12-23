use crate::core::Context;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::broadcast;

pub struct Watch {
    ctx: Context,
    watcher: RecommendedWatcher,
    tx: broadcast::Sender<Event>,
}

#[derive(Debug, Clone)]
pub enum Event {
    One(String),
    All,
}

impl Watch {
    pub fn new(ctx: Context) -> Self {
        let (tx, _) = broadcast::channel(16);
        let foo = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                match res {
                    Ok(event) => {
                        eprintln!("event: {:?}", event);
                        match tx.send(Event::All) {
                            Ok(_) => (),
                            Err(e) => eprintln!("channel send error: {e}"),
                        }
                    }
                    Err(error) => eprintln!("error: {}", error),
                };
                ()
            },
            Config::default(),
        )
        .unwrap();

        watcher
            .watch(&ctx.src_dir, RecursiveMode::Recursive)
            .unwrap();

        Self {
            ctx,
            watcher,
            tx: foo,
        }
    }
}

#[tokio::main]
pub async fn blarg(ctx: Context) {
    let watch = Watch::new(ctx);

    let mut rx = watch.tx.subscribe();
    tokio::spawn(async move {
        loop {
            let event = rx.recv().await.unwrap();
            dbg!(event);
        }
    })
    .await
    .unwrap();
}
