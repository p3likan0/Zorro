use axum::{
    response::Json,
    routing::get,
    routing::post,
    Router
};


mod package;


#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/v1/packages", get(get_packages))
                           .route("/v1/packages/upload/:package_name", post(package::upload_package));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    package::create_uploads_directory().await.expect("Could not create uploads directory");
    axum::serve(listener, app).await.unwrap();
}

async fn get_packages() -> Json<package::Package> {
    let p = package::Package{name: "python3".to_string(), version: "1.2.3".to_string(), hash: "aoaeuaoue".to_string()};
    Json(p)
}
