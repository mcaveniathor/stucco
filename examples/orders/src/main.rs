#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let port = std::env::var("PORT").unwrap_or_else(|_| "4181".into());
    let path = std::path::PathBuf::from(
        std::env::var("ORDERS_DB").unwrap_or_else(|_| "target/orders.redb".into()),
    );
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let origin =
        std::env::var("ORDERS_ORIGIN").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let app = orders::app(&path, stucco::server::SecretKey::from_env()?, &origin)?;
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    // Print the trusted origin: forms posted from any other address get 403.
    println!("orders listening on {origin}/orders");
    axum::serve(listener, app).await?;
    Ok(())
}
