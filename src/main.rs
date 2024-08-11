use axum::{
    routing::get,
    routing::post,
    Router
};

mod package;
mod repository;

use std::sync::Arc;

const CONFIG_PATH: &str = ".config/repository_structure.yaml";

#[tokio::main]
async fn main() {
    let archive = repository::DebianArchive::new(CONFIG_PATH);

    let shared_archive = Arc::new(archive); 

    let app = Router::new().route("/v1/packages", get(package::get_packages))
                           .route("/v1/packages/upload/:package_name", post(package::upload_package))
                           .route("/v1/repositories", get(repository::handle_get_repositories)).with_state(shared_archive);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    package::create_uploads_directory().await.expect("Could not create uploads directory");
    axum::serve(listener, app).await.unwrap();
}
