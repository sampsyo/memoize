use crate::Context;
use crate::core::Resource;
use crate::watch::Watch;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response, sse},
    routing::get,
};
use axum_extra::body::AsyncReadBody;
use std::convert::Infallible;
use std::path;
use std::sync::Arc;
use tokio::fs;
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};

struct AppState {
    ctx: Context,
    watch: Watch,
}

#[tokio::main]
pub async fn serve(ctx: Context) {
    let watch = Watch::new(&ctx.src_dir);
    let state = Arc::new(AppState { ctx, watch });
    let app = Router::new()
        .route("/_notify", get(notify))
        .route("/{*path}", get(resource))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    eprintln!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

/// Respond with the contents of a file on the filesystem.
async fn send_file(path: &path::Path) -> Result<Response, (StatusCode, String)> {
    let mime = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or(mime_guess::mime::OCTET_STREAM.as_str());

    let file = fs::File::open(path)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("not found: {e}")))?;

    let headers = [(header::CONTENT_TYPE, mime)];
    let body = AsyncReadBody::new(file);
    Ok((headers, body).into_response())
}

/// Serve a resource from the site.
async fn resource(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let ctx = &state.ctx;
    eprintln!("GET {path}");
    match ctx.resolve_resource(&path) {
        Some(Resource::Note(src_path)) => {
            let mut buf: Vec<u8> = vec![];
            match ctx.render_note(&src_path, &mut buf) {
                Ok(()) => Ok(Html(buf).into_response()),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("note rendering failed: {e}"),
                )),
            }
        }
        Some(Resource::Static(src_path)) => send_file(&src_path).await,
        Some(Resource::Directory(_)) => Err((
            StatusCode::NOT_IMPLEMENTED,
            "directory listings not implemented".into(),
        )),
        None => Err((StatusCode::NOT_FOUND, "not found".into())),
    }
}

// Server-Sent Events endpoint for getting change notifications.
async fn notify(
    State(state): State<Arc<AppState>>,
) -> sse::Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let rx = state.watch.channel.subscribe();
    let stream = BroadcastStream::new(rx).map(|_| Ok(sse::Event::default().data("reload")));
    sse::Sse::new(stream)
}
