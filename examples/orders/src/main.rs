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
    let app = orders::app(&path)?;
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    println!("orders listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
