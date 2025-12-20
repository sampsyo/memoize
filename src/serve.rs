use crate::Context;
use axum::{Router, extract::State, http::Uri};
use std::sync::Arc;

#[tokio::main]
pub async fn serve(ctx: Context) {
    tracing_subscriber::fmt::init();

    let shared_ctx = Arc::new(ctx);
    let app = Router::new().fallback(blarg).with_state(shared_ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn blarg(State(ctx): State<Arc<Context>>, uri: Uri) -> &'static str {
    tracing::info!("request for {}", uri.path());
    dbg!(&ctx.dest_dir);
    "Hello, World!"
}
