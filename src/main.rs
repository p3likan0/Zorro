use zorro_lib;

const BASE_URL: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() {
    zorro_lib::run_server(BASE_URL).await;
}
