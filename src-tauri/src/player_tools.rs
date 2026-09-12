use axum::{
    body::Body,
    extract::Path,
    http::{header, Response, StatusCode},
    response::IntoResponse,
};

const WINDOWS: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/stickplay-player-tools-windows.zip"
));
const MACOS: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/stickplay-player-tools-macos.zip"
));

pub async fn download(Path(platform): Path<String>) -> Response<Body> {
    let (contents, filename) = match platform.as_str() {
        "windows" => (WINDOWS, "stickplay-player-tools-windows.zip"),
        "macos" => (MACOS, "stickplay-player-tools-macos.zip"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CONTENT_LENGTH, contents.len())
        .body(Body::from(contents))
        .expect("fixed player tool response headers must be valid")
}
