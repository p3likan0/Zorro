use axum::{
    extract::{Request},
    body::Bytes,
    http::StatusCode,
    //response::Json,
    extract::State,
    BoxError,
};
use std::{path, path::PathBuf};
use futures::{Stream, TryStreamExt};
use tokio_util::io::StreamReader;
use tokio::{io::BufWriter};
use std::{io, io::{Error, ErrorKind::{Other, InvalidInput}}};
use crate::repository::RepositoryConfig;
use std::sync::Arc;

mod binary_package;
mod hash_utils;

pub fn create_directories(config: &RepositoryConfig) -> io::Result<()> {
    println!("Creating directories: {}, {}", &config.uploads_dir, &config.pool_dir);
    let uploads_dir = path::Path::new(&config.uploads_dir);
    let pool_dir = path::Path::new(&config.pool_dir);
    std::fs::create_dir_all(uploads_dir)?;
    std::fs::create_dir_all(pool_dir)
}

pub async fn handle_upload_package(
    State(config): State<Arc<RepositoryConfig>>,
    axum::extract::Path(package_name): axum::extract::Path<String>,
    request: Request,
) -> Result<(), (StatusCode, String)> {
    validate_upload_package_name(&package_name).
        map_err(|err| {
            eprintln!("Error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })?;

    // Stream to file
    let path = std::path::Path::new(&config.uploads_dir).join(&package_name);
    stream_to_file(&path, request.into_body().into_data_stream()).await?;

    binary_package::DebianBinaryPackage::process(&path, &config.pool_dir)
        .map_err(|err| {
            eprintln!("Error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process debian package".to_string())
        })?;
    Ok(())
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
fn validate_upload_package_name(path: &str) -> io::Result<()> {
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return Err(Error::new(InvalidInput, "Package name is invalid".to_string()));
        }
    }

    if components.count() != 1 {
        return Err(Error::new(InvalidInput, "Package name is invalid".to_string()));
        //return Err((StatusCode::INTERNAL_SERVER_ERROR, "Package name is invalid".to_string()));
    }
    Ok(())
}

async fn stream_to_file<S, E>(path: &PathBuf, stream: S) -> Result<(), (StatusCode, String)>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    println!("receiving: {}",path.display());
    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(|err| io::Error::new(Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        let mut file = BufWriter::new(tokio::fs::File::create(path).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

//pub async fn get_packages() -> Json<Package> {
//    //let p = Package{package_name: "python3".to_string(), version: "1.2.3".to_string(), md5: "aoaeuaoue".to_string()};
//    Json()
//}
