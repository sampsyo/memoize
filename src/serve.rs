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
use std::sync::{Arc, RwLock};
use tokio::fs;
use tokio_stream::{Stream, StreamExt};

#[derive(Clone)]
struct AppState {
    ctx: Arc<RwLock<Context>>,
    watch: Arc<Watch>,
}

#[tokio::main]
pub async fn serve(ctx: Context) {
    // Watch the source directory and, in debug mode, the templates directory.
    let watch = Watch::new(&[
        &ctx.src_dir,
        #[cfg(debug_assertions)]
        path::Path::new(crate::core::TEMPLATES.dir),
    ]);
    let state = AppState {
        ctx: Arc::new(RwLock::new(ctx)),
        watch: Arc::new(watch),
    };

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
#[axum::debug_handler]
async fn resource(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    eprintln!("GET {path}");

    let rsrc = {
        let ctx = &mut state.ctx.read().unwrap();
        ctx.resolve_resource(&path)
    };
    match rsrc {
        Some(Resource::Note(src_path)) => {
            // In debug mode, reload templates before rendering.
            #[cfg(debug_assertions)]
            state.ctx.write().unwrap().reload_templates();

            // Render and send the note.
            let mut buf: Vec<u8> = vec![];
            match state.ctx.read().unwrap().render_note(&src_path, &mut buf) {
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

/// Server-Sent Events endpoint for getting change notifications.
async fn notify(
    State(state): State<AppState>,
) -> sse::Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let stream = state.watch.stream().map(|_| {
        eprintln!("sending reload event");
        Ok(sse::Event::default().event("reload").data("_"))
    });
    sse::Sse::new(stream)
}
