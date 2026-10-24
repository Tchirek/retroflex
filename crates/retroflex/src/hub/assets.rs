use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

// 綴じ込みし　画面ひとそろい　懐に
#[derive(Embed)]
#[folder = "../../app/dist"]
#[allow_missing = true]
struct Web;

// 道問えば　知らぬ角でも　玄関へ
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let (path, file) = match Web::get(path).filter(|_| !path.is_empty()) {
        Some(f) => (path, f),
        None if path.starts_with("api/") => return StatusCode::NOT_FOUND.into_response(),
        None => match Web::get("index.html") {
            Some(f) => ("index.html", f),
            None => return (StatusCode::NOT_FOUND, "web app not built").into_response(),
        },
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    ([(header::CONTENT_TYPE, mime.as_ref()), (header::CACHE_CONTROL, cache)], file.data).into_response()
}
