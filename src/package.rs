use axum::{
    extract::{Path, Request},
    body::Bytes,
    http::StatusCode,
    response::Json,
    extract::State,
    BoxError,
};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use futures::{Stream, TryStreamExt};
use tokio_util::io::StreamReader;
use tokio::{io::BufWriter};
use std::io;

use crate::repository::RepositoryConfig;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub hash: String
}


pub fn create_directories(config: &crate::repository::RepositoryConfig) -> std::io::Result<()> {
    println!("Creating directories: {}, {}", &config.uploads_dir, &config.pool_dir);
    let uploads_dir = std::path::Path::new(&config.uploads_dir);
    let pool_dir = std::path::Path::new(&config.pool_dir);
    std::fs::create_dir_all(uploads_dir)?;
    std::fs::create_dir_all(pool_dir)
}

// We move the package using rename, which brings the limitation of the file needing to be in the
// same filesystem but is extremely fast.
fn move_package_to_pool(deb_path: &PathBuf, pool_dir: &str) -> Result<(), (StatusCode, String)> {
    let dest_dir: PathBuf;
    let file_name_str = deb_path.file_name().expect("Could not decode package name")
        .to_str().expect("Could not decode package name to string");
    // Since there are going to be a lot of libx_1_2_3_arch.deb packages, we crate a subdirectory
    // for each one. Ex /lib/a/liba_1_2_3_amd64.deb, /lib/b/libb_1_2_3_amd64.deb
    if file_name_str.starts_with("lib"){
        let lib_fourth_char = file_name_str.chars().nth(3).expect("Library package does not contain a valid name");
        dest_dir = std::path::Path::new(&pool_dir).join("lib").join(lib_fourth_char.to_string());
    } else {
        let pkg_first_char = file_name_str.chars().nth(0).expect("Package does not contain a valid name");
        dest_dir = std::path::Path::new(&pool_dir).join(pkg_first_char.to_string());
    }
    std::fs::create_dir_all(&dest_dir).map_err(|err| {
        eprintln!("Error creating dir: {}, error: {}", dest_dir.display(), err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to move package".to_string())
    })?;
    std::fs::rename(deb_path, dest_dir.join(file_name_str)).map_err(|err| {
        eprintln!("Error renaming deb from: {}, to: {}, error: {}", deb_path.display(), dest_dir.display(), err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to move package".to_string())
    })?;
    Ok(())
}

fn validate_package(deb_path: &PathBuf) -> Result<(), (StatusCode, String)> {
    // Open the Debian package file
    let deb_file = std::fs::File::open(deb_path)
        .map_err(|err| {
            eprintln!("Failed to open file {}: {}", deb_path.display(), err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open package file".to_string())
        })?;

    // Parse the Debian package
    let mut pkg = debpkg::DebPkg::parse(deb_file)
        .map_err(|err| {
            eprintln!("Package {} is not valid, error: {}", deb_path.display(), err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse package".to_string())
        })?;

    // Extract and parse the control file
    let control_tar = pkg.control()
        .map_err(|err| {
            eprintln!("Package {} is not valid, error: {}", deb_path.display(), err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to extract control tarball".to_string())
        })?;

    let control = debpkg::Control::extract(control_tar)
        .map_err(|err| {
            eprintln!("Package {} is not valid, error: {}", deb_path.display(), err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to extract control data".to_string())
        })?;

    //println!("Package Name: {:#?}", control);
    println!("Package: {}, version: {}", control.name() ,control.version());
    Ok(())
}

pub async fn handle_upload_package(
    State(config): State<Arc<RepositoryConfig>>,
    Path(package_name): Path<String>,
    request: Request,
) -> Result<(), (StatusCode, String)> {
    validate_package_name(&package_name)?;
    let path = std::path::Path::new(&config.pool_dir).join(&package_name);
    stream_to_file(&path, request.into_body().into_data_stream()).await?;
    validate_package(&path)?;
    move_package_to_pool(&path, &config.pool_dir)
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
fn validate_package_name(path: &str) -> Result<(), (StatusCode, String)> {
    let path = std::path::Path::new(path);
    let mut components = path.components().peekable();

    if let Some(first) = components.peek() {
        if !matches!(first, std::path::Component::Normal(_)) {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Package name is invalid".to_string()));
        }
    }

    if components.count() != 1 {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Package name is invalid".to_string()));
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
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        let mut file = BufWriter::new(tokio::fs::File::create(path).await?);
        println!("HANDLEEER");

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

pub async fn get_packages() -> Json<Package> {
    let p = Package{name: "python3".to_string(), version: "1.2.3".to_string(), hash: "aoaeuaoue".to_string()};
    Json(p)
}
