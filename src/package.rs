use axum::{
    extract::{Path, Request},
    body::Bytes,
    http::StatusCode,
    response::Json,
    BoxError,
};
use serde::{Deserialize, Serialize};
use futures::{Stream, TryStreamExt};
use tokio_util::io::StreamReader;
use tokio::{fs::File, io::BufWriter};
use std::io;

#[derive(Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub hash: String
}

const UPLOADS_DIRECTORY: &str = "uploads";

pub async fn create_uploads_directory() -> std::io::Result<()> {
    tokio::fs::create_dir_all(UPLOADS_DIRECTORY).await
}

pub async fn upload_package(
    Path(package_name): Path<String>,
    request: Request,
) -> Result<(), (StatusCode, String)> {
    stream_to_file(&package_name, request.into_body().into_data_stream()).await
}

async fn stream_to_file<S, E>(path: &str, stream: S) -> Result<(), (StatusCode, String)>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    if !path_is_valid(path) {
        return Err((StatusCode::BAD_REQUEST, "Invalid path".to_owned()));
    }
    println!("File arriving: {}", path);

    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Create the file. `File` implements `AsyncWrite`.
        let path = std::path::Path::new(UPLOADS_DIRECTORY).join(path);
        let mut file = BufWriter::new(File::create(path).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
fn path_is_valid(path: &str) -> bool {
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return false;
        }
    }

    components.count() == 1
}

pub async fn get_packages() -> Json<Package> {
    let p = Package{name: "python3".to_string(), version: "1.2.3".to_string(), hash: "aoaeuaoue".to_string()};
    Json(p)
}
