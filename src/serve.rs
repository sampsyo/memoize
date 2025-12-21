use crate::Context;
use axum::{Router, extract::State, http::Uri};
use std::path::{Component, Path, PathBuf};
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

/// Validate and relative-ize a requested path. If we return a path, it is now
/// safe to `join` with a base directory without "escaping" that directory. May
/// return `None` for any disallowed path.
///
/// Inspired by `build_and_validate_path` in tower-http's `ServeDir` service.
fn sanitize_path(path: &str) -> Option<PathBuf> {
    // TODO percent-decode?

    let mut path_buf = PathBuf::new();
    for comp in Path::new(path).components() {
        match comp {
            Component::Normal(c) => path_buf.push(c), // Normal filename.
            Component::ParentDir => return None,      // Disallow `..`.
            Component::Prefix(_) => return None,      // Disallow `C:`.
            Component::RootDir => (),                 // Strip leading `/`.
            Component::CurDir => (),                  // Ignore `.`.
        }
    }

    Some(path_buf)
}

async fn blarg(State(ctx): State<Arc<Context>>, uri: Uri) -> &'static str {
    let path = sanitize_path(uri.path());
    if let Some(path) = path {
        tracing::info!("request for {:?}", &path);
        let src_path = ctx.src_dir.join(&path);
        dbg!(src_path);
        "Hello, World!"
    } else {
        "not found"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute() {
        assert_eq!(sanitize_path("/hi.txt"), Some("hi.txt".into()));
    }

    #[test]
    fn relative() {
        assert_eq!(sanitize_path("hi.txt"), Some("hi.txt".into()));
    }

    #[test]
    fn with_dir() {
        assert_eq!(sanitize_path("/dir/hi.txt"), Some("dir/hi.txt".into()));
    }

    #[test]
    fn dot_dot() {
        assert_eq!(sanitize_path("/../hi.txt"), None);
    }
}
