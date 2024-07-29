use axum::Router;

#[tokio::main]
async fn main() {
    // tokio::spawn(async move { game_server(game_request_rx, 5).await });

    // build our application with a single route

    let assets_dir = std::path::PathBuf::from("../frontend/");

    let app: Router = Router::new().fallback_service(axum::routing::get_service(
        tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
    ));
    // .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
