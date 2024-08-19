use axum::{
    routing::get,
    routing::post,
    Router
};

mod repository;
mod packages;
mod release;
mod database;


use std::sync::Arc;

const CONFIG_PATH: &str = ".config/repository_structure.yaml";
const PUBLISH_PATH: &str = "/tmp/publish";

pub async fn run_server(base_url: &str) {
    let listener = tokio::net::TcpListener::bind(base_url)
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app(CONFIG_PATH)).await.unwrap();
}

fn app(config_path: &str) -> Router {
    let archive = repository::Repository::new(config_path).unwrap();
    for (suite, distribution) in &archive.config.dists {
        let release = release::DebianRelease::new(
            suite.to_string(),
            distribution.components.clone(),
            distribution.version.to_string(),
            distribution.origin.to_string(),
            distribution.label.to_string(),
            distribution.architectures.clone(),
            distribution.description.to_string(),
            distribution.codename.to_string(),
            );
        release.save_to_file(PUBLISH_PATH).expect("could not save to file");
    }

    database::create_tables(&archive.db_conn).unwrap();
    database::insert_distributions(&archive.db_conn, &archive.config.dists).unwrap();
    packages::create_directories(&archive.config).expect("Could not create uploads directory"); // Not tested yet

    let shared_archive = Arc::new(archive); 

    Router::new()
        .route("/v1/packages", get(packages::handle_get_package_name_version_arch))
        .route("/v1/packages/upload/:package_name", post(packages::handle_upload_package))
        .route("/v1/repositories", get(repository::handle_get_repositories))
        .with_state(shared_archive)
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::{ServiceExt}; // for `call`, `oneshot`, and `ready`
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tempdir::TempDir;

    #[tokio::test]
    async fn handler_get_packages() {
        let (config, _tmp_dir, app) = test_setup();

        let deb_orig_contents = std::fs::read("tests/packages/hello_2.10-2_amd64.deb").expect("Failed to test package");
        let response = app.clone()
            .oneshot(Request::builder()
                .method(axum::http::Method::POST)
                .uri("/v1/packages/upload/hello_2.10-2_amd64.deb")
                .body(Body::from(deb_orig_contents.clone())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(Request::builder().uri("/v1/packages?name=hello&version=2.10-2&arch=amd64").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        let expected_json = json!({
                "control": {
                    "architecture": "amd64",
                    "breaks": "hello-debhelper (<< 2.9)",
                    "built_using": null,
                    "conflicts": "hello-traditional",
                    "depends": "libc6 (>= 2.14)",
                    "description": "The GNU hello program produces a familiar, friendly greeting.  It\nallows non-programmers to use a classic computer science tool which\nwould otherwise be unavailable to them.\n.\nSeriously, though: this is an example of how to do a Debian package.\nIt is the Debian version of the GNU Project's `hello world' program\n(which is itself an example for the GNU Project).",
                    "enhances": null,
                    "essential": null,
                    "homepage": "http://www.gnu.org/software/hello/",
                    "installed_size": "280",
                    "maintainer": "Santiago Vila <sanvila@debian.org>",
                    "package": "hello",
                    "pre_depends": null,
                    "priority": "optional",
                    "provides": null,
                    "recommends": null,
                    "replaces": "hello-debhelper (<< 2.9), hello-traditional",
                    "section": "devel",
                    "source": null,
                    "suggests": null,
                    "version": "2.10-2"
                },
              "description_md5": null,
              "filename": format!("{}/h/hello_2.10-2_amd64.deb", config.pool_dir),
              "md5sum": "52b0cad2e741dd722c3e2e16a0aae57e",
              "sha1": "9942852719b998fb190848966bcbe13f10534842",
              "sha256": "35b1508eeee9c1dfba798c4c04304ef0f266990f936a51f165571edf53325cbc",
              "size": 56132
        });
        assert_eq!(body, expected_json);
    }
    #[tokio::test]
    async fn handler_upload_package() {
        let (config, _tmp_dir, app) = test_setup();

        let deb_orig_contents = std::fs::read("tests/packages/hello_2.10-2_amd64.deb").expect("Failed to test package");

        let response = app
            .oneshot(Request::builder()
                .method(axum::http::Method::POST)
                .uri("/v1/packages/upload/hello_2.10-2_amd64.deb")
                .body(Body::from(deb_orig_contents.clone())).unwrap())
            .await
            .unwrap();
        let expected_deb = std::path::PathBuf::from(config.pool_dir).join("h/hello_2.10-2_amd64.deb");
        assert!(expected_deb.exists());
        let deb_uploaded_contents = std::fs::read(expected_deb).expect("Failed to read uploaded file");
        assert_eq!(&deb_orig_contents, &deb_uploaded_contents);
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn handler_upload_library_package() {
        let (config, _tmp_dir, app) = test_setup();

        let deb_orig_contents = std::fs::read("tests/packages/libsqlite0_2.8.17-15+deb10u1_amd64.deb").expect("Failed to test package");

        let response = app
            .oneshot(Request::builder()
                .method(axum::http::Method::POST)
                .uri("/v1/packages/upload/libsqlite0_2.8.17-15+deb10u1_amd64.deb")
                .body(Body::from(deb_orig_contents.clone())).unwrap())
            .await
            .unwrap();
        let expected_deb = std::path::PathBuf::from(config.pool_dir).join("lib/s/libsqlite0_2.8.17-15+deb10u1_amd64.deb");
        assert!(expected_deb.exists());
        let deb_uploaded_contents = std::fs::read(expected_deb).expect("Failed to read uploaded file");
        assert_eq!(&deb_orig_contents, &deb_uploaded_contents);
        assert_eq!(response.status(), StatusCode::OK);
    }

    fn test_setup() -> (repository::RepositoryConfig, TempDir, Router){
        let tmp_dir = TempDir::new("test").unwrap();
        let mut config = repository::RepositoryConfig::new("tests/repository_structure_1.yml").unwrap();
        config.db_file = tmp_dir.path().join("test_file.db").into_os_string().into_string().unwrap();
        config.uploads_dir = tmp_dir.path().join("uploads").into_os_string().into_string().unwrap();
        config.pool_dir = tmp_dir.path().join("pool").into_os_string().into_string().unwrap();
        let config_file = tmp_dir.path().join("config.yml");
        config.write_to_file(&config_file).unwrap();
        let app = app(&config_file.into_os_string().into_string().unwrap());
        (config, tmp_dir, app)
    }
    #[tokio::test]
    async fn handler_get_repositories() {
        let (config, _tmp_dir, app) = test_setup();
        
        let response = app
            .oneshot(Request::builder().uri("/v1/repositories").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        //println!("{:#}", body);
        let expected_json = json!({
            "db_file": config.db_file,
            "uploads_dir": config.uploads_dir,
            "pool_dir": config.pool_dir,
            "dists": {
              "stable": {
                "architectures": [
                  "arm64",
                  "amd64"
                ],
                "codename": "codename",
                "description": "this is a distribution description",
                "label": "label",
                "origin": "origin",
                "components": [
                  "main",
                  "contrib",
                  "testing"
                ],
                "version": "version"
              },
              "unstable": {
                "architectures": [
                  "arm64",
                  "amd64"
                ],
                "codename": "codename",
                "description": "this is a distribution description",
                "label": "label",
                "origin": "origin",
                "components": [
                  "main",
                  "contrib",
                  "testing"
                ],
                "version": "version"
              }
            }
        });
        assert_eq!(body, expected_json);
    }
}
