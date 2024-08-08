use axum::{response::Json, routing::get, Router};

mod package;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/packages", get(get_packages));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn get_packages() -> Json<package::Package> {
    let p = package::Package{name: "python3".to_string(), version: "1.2.3".to_string(), hash: "aoaeuaoue".to_string()};
    Json(p)
}
