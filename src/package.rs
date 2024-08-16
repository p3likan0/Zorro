use axum::{
    extract::{Request},
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
use std::{io, io::{Error, ErrorKind::{InvalidData, InvalidInput}}};
use debpkg::Control;
use crate::repository::RepositoryConfig;
use std::sync::Arc;
use std::path;

#[derive(Debug, Serialize, Deserialize)]
struct Package {
    key: String,
    package_name: String,
    version: String,
    architecture: String,
    md5: String,
    maintainer: Option<String>,
    description: Option<String>,
    depends: Option<String>,
    recommends: Option<String>,
    suggests: Option<String>,
    enhances: Option<String>,
    pre_depends: Option<String>,
    breaks: Option<String>,
    conflicts: Option<String>,
    provides: Option<String>,
    replaces: Option<String>,
    installed_size: Option<String>,
    homepage: Option<String>,
    source: Option<String>,
    section: Option<String>,
    priority: Option<String>,
}

impl Package {
    fn read_control(deb_path: &path::Path) -> io::Result<debpkg::Control> {
        // Open the Debian package file
        let deb_file = std::fs::File::open(deb_path)?;

        // Parse the Debian package
        let mut pkg = debpkg::DebPkg::parse(deb_file)
            .map_err(|err| {
                Error::new(InvalidData ,format!("Failed to parse package {}, error: {}", deb_path.display(), err))
            })?;

        // Extract and parse the control file
        let control_tar = pkg.control()
            .map_err(|err| {
                Error::new(InvalidData ,format!("Control for package {} not valid, error: {}", deb_path.display(), err))
            })?;

        let control = Control::extract(control_tar)
            .map_err(|err| {
                Error::new(InvalidData ,format!("Cannot extract control data from package {}, error {}", deb_path.display(), err))
            })?;
        Ok(control)
    }

    fn new_from_control(control: &debpkg::Control, md5: &str) -> io::Result<Package> { 
        let arch = match control.get("Architecture") {
            Some(arch) => arch,
            None => return Err(Error::new(InvalidData, format!("Cannot get the architecture from the package: {} {}", control.name(), control.version())))
        };

        let key = format!("{} {} {} {}", control.name(), control.version(), arch, md5);
        Ok(Package{
            key: key, 
            package_name: control.name().to_string(),
            version: control.version().to_string(),
            architecture: arch.to_string(),
            md5: md5.to_string(),
            maintainer: control.get("Maintainer").map(|s| s.to_string()),
            description: control.long_description().map(|s| s.to_string()),
            depends: control.get("Depends").map(|s| s.to_string()),
            recommends: control.get("Recommends").map(|s| s.to_string()),
            suggests: control.get("Suggests").map(|s| s.to_string()),
            enhances: control.get("Enhances").map(|s| s.to_string()),
            pre_depends: control.get("Pre-Depends").map(|s| s.to_string()),
            breaks: control.get("Breaks").map(|s| s.to_string()),
            conflicts: control.get("Conflicts").map(|s| s.to_string()),
            provides: control.get("Provides").map(|s| s.to_string()),
            replaces: control.get("Replaces").map(|s| s.to_string()),
            installed_size: control.get("Installed-Size").map(|s| s.to_string()),
            homepage: control.get("Homepage").map(|s| s.to_string()),
            source: control.get("Source").map(|s| s.to_string()),
            section: control.get("Section").map(|s| s.to_string()),
            priority: control.get("Priority").map(|s| s.to_string()),
        })
    }
}

pub fn create_directories(config: &crate::repository::RepositoryConfig) -> io::Result<()> {
    println!("Creating directories: {}, {}", &config.uploads_dir, &config.pool_dir);
    let uploads_dir = path::Path::new(&config.uploads_dir);
    let pool_dir = path::Path::new(&config.pool_dir);
    std::fs::create_dir_all(uploads_dir)?;
    std::fs::create_dir_all(pool_dir)
}

// We move the package using rename, which brings the limitation of the file needing to be in the
// same filesystem but is extremely fast.
fn move_package_to_pool(deb_path: &path::Path, pool_dir: &str) -> Result<(), (StatusCode, String)> {
    let dest_dir: PathBuf;
    let file_name_str = deb_path.file_name().expect("Could not decode package name")
        .to_str().expect("Could not decode package name to string");
    // Since there are going to be a lot of libx_1_2_3_arch.deb packages, we crate a subdirectory
    // for each one. Ex /lib/a/liba_1_2_3_amd64.deb, /lib/b/libb_1_2_3_amd64.deb
    if file_name_str.starts_with("lib"){
        let lib_fourth_char = file_name_str.chars().nth(3).expect("Library package does not contain a valid name");
        dest_dir = path::Path::new(&pool_dir).join("lib").join(lib_fourth_char.to_string());
    } else {
        let pkg_first_char = file_name_str.chars().nth(0).expect("Package does not contain a valid name");
        dest_dir = path::Path::new(&pool_dir).join(pkg_first_char.to_string());
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

pub async fn handle_upload_package(
    State(config): State<Arc<RepositoryConfig>>,
    axum::extract::Path(package_name): axum::extract::Path<String>,
    request: Request,
) -> Result<(), (StatusCode, String)> {
    validate_package_name(&package_name).
        map_err(|err| {
            eprintln!("Error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })?;

    // Stream to file
    let path = std::path::Path::new(&config.pool_dir).join(&package_name);
    stream_to_file(&path, request.into_body().into_data_stream()).await?;

    // Read control to validate the package
    let control = Package::read_control(&path)
        .map_err(|err| {
            eprintln!("Error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to extract control data".to_string())
        })?;
    println!("Package: {}, version: {}", control.name() ,control.version());

    move_package_to_pool(&path, &config.pool_dir)?;
    let package = Package::new_from_control(&control, "12345");
    println!("package: {:#?}", package);
    Ok(())
}

// to prevent directory traversal attacks we ensure the path consists of exactly one normal
// component
fn validate_package_name(path: &str) -> io::Result<()> {
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
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
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
