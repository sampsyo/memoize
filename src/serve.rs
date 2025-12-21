use crate::Context;
use crate::core::Resource;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_extra::body::AsyncReadBody;
use std::path;
use std::sync::Arc;
use tokio::fs;

#[tokio::main]
pub async fn serve(ctx: Context) {
    let shared_ctx = Arc::new(ctx);
    let app = Router::new()
        .route("/{*path}", get(handle))
        .with_state(shared_ctx);

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
async fn handle(
    State(ctx): State<Arc<Context>>,
    Path(path): Path<String>,
) -> Result<Response, (StatusCode, String)> {
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
