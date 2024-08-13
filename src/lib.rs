use axum::{
    routing::get,
    routing::post,
    Router
};

mod package;
mod repository;
mod release;

use std::sync::Arc;

const CONFIG_PATH: &str = ".config/repository_structure.yaml";
const PUBLISH_PATH: &str = "/tmp/publish";

pub async fn run_server(base_url: &str) {
    package::create_uploads_directory().await.expect("Could not create uploads directory"); // Not tested yet
    let listener = tokio::net::TcpListener::bind(base_url)
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app(CONFIG_PATH)).await.unwrap();
}

fn app(config_path: &str) -> Router {
    let archive = repository::RepositoryConfig::new(config_path);
    for (suite, distribution) in &archive.dists {
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


    let shared_archive = Arc::new(archive); 

    Router::new().route("/v1/packages", get(package::get_packages))
        .route("/v1/packages/upload/:package_name", post(package::upload_package))
        .route("/v1/repositories", get(repository::handle_get_repositories)).with_state(shared_archive)

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

    #[tokio::test]
    async fn handler_get_packages() {
        let app = app(CONFIG_PATH);

        let response = app
            .oneshot(Request::builder().uri("/v1/packages").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

    }
    #[tokio::test]
    async fn handler_get_repositories() {
        let app = app("tests/repository_structure_1.yml");

        let response = app
            .oneshot(Request::builder().uri("/v1/repositories").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        println!("{:#}", body);
        let expected_json = json!({
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
