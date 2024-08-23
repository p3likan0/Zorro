use crate::repository::{Repository, RepositoryConfig};
use axum::{
    body::Bytes,
    extract::State,
    extract::{Query, Request},
    http::StatusCode,
    response::{IntoResponse, Json},
    BoxError,
};
use futures::{Stream, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    io,
    io::{
        Error,
        ErrorKind::{InvalidInput, Other},
    },
};
use std::{path, path::PathBuf};
use tokio::io::BufWriter;
use tokio_util::io::StreamReader;

use serde_json::json;
pub mod binary_package;
mod hash_utils;
use derive_more::Display;

use crate::database;

pub fn create_directories(config: &RepositoryConfig) -> io::Result<()> {
    println!(
        "Creating directories: {}, {}",
        &config.uploads_dir, &config.pool_dir
    );
    let uploads_dir = path::Path::new(&config.uploads_dir);
    let pool_dir = path::Path::new(&config.pool_dir);
    std::fs::create_dir_all(uploads_dir)?;
    std::fs::create_dir_all(pool_dir)
}

pub async fn handle_upload_package(
    State(repo): State<Arc<Repository>>,
    axum::extract::Path(package_name): axum::extract::Path<String>,
    request: Request,
) -> Result<(), (StatusCode, String)> {
    validate_upload_package_name(&package_name).map_err(|err| {
        eprintln!("Error: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
    })?;

    // Stream to file
    let path = std::path::Path::new(&repo.config.uploads_dir).join(&package_name);
    stream_to_file(&path, request.into_body().into_data_stream()).await?;

    binary_package::DebianBinaryPackage::process(&path, &repo.config.pool_dir, &repo.db_conn)
        .map_err(|err| {
            eprintln!("Error: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to process debian package".to_string(),
            )
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
            return Err(Error::new(
                InvalidInput,
                "Package name is invalid".to_string(),
            ));
        }
    }

    if components.count() != 1 {
        return Err(Error::new(
            InvalidInput,
            "Package name is invalid".to_string(),
        ));
        //return Err((StatusCode::INTERNAL_SERVER_ERROR, "Package name is invalid".to_string()));
    }
    Ok(())
}

async fn stream_to_file<S, E>(path: &PathBuf, stream: S) -> Result<(), (StatusCode, String)>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    println!("receiving: {}", path.display());
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

#[derive(Debug, Deserialize, Serialize, Display, Clone)]
#[display(
    fmt = "DistributionKey: name = {}, version = {}, architecture: {}",
    name,
    version,
    architecture
)]
pub struct PackageKey {
    pub name: String,
    pub version: String,
    pub architecture: String,
}

pub async fn handle_get_package_name_version_arch(
    State(repo): State<Arc<Repository>>,
    Query(query): Query<PackageKey>,
) -> impl IntoResponse {
    // Maybe is better to receive this a json instead of go with query
    let pkg = PackageKey {
        name: query.name,
        version: query.version,
        architecture: query.architecture,
    };
    match database::get_debian_binary_package(&repo.db_conn, &pkg) {
        Ok(package) => (StatusCode::OK, Json(package)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": format!("{}", err)
            })),
        )
            .into_response(),
    }
}
