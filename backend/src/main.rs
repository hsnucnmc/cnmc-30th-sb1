use std::collections::BTreeSet;
use std::future::IntoFuture;

use axum::{routing::get, Router};

use tokio::sync::{mpsc, watch};

use train_backend::{handler, packet::*, train, AppState};

#[tokio::main]
async fn main() {
    loop {
        let (view_request_tx, view_request_rx) = mpsc::channel(32);

        let (ctrl_request_tx, ctrl_request_rx) = mpsc::channel(32);

        let (valid_id_tx, valid_id_rx) = watch::channel(BTreeSet::new());

        let (derail_tx, derail_rx) = mpsc::channel(1);

        let tm_handle = tokio::spawn(async move {
            train::train_master(view_request_rx, ctrl_request_rx, valid_id_tx, derail_rx).await
        });

        // build our application with a single route

        let shared_state = AppState {
            view_request_tx,
            valid_id: valid_id_rx,
            ctrl_request_tx,
            derail_tx,
        };

        let assets_dir = std::path::PathBuf::from("../frontend/");

        use tower_http::body::Full;
        let app: Router = Router::new()
            .fallback_service(axum::routing::get_service(
                tower_http::services::ServeDir::new(assets_dir)
                    .append_index_html_on_directories(true)
                    .fallback(tower_http::services::redirect::Redirect::<Full>::temporary("/".parse().unwrap())),
            ))
            .route(
                "/derailer",
                axum::routing::get_service(tower_http::services::ServeFile::new(
                    "../frontend/derailer.html",
                )),
            )
            .route(
                "/list",
                axum::routing::get_service(tower_http::services::ServeFile::new(
                    "../frontend/list.html",
                )),
            )
            .route(
                "/control",
                axum::routing::get_service(tower_http::services::ServeFile::new(
                    "../frontend/control.html",
                )),
            )
            .route("/ws", get(handler::ws_get_handler))
            .route("/ws-ctrl", get(handler::ctrl_get_handler))
            .route("/force-derail", get(handler::derail_handler))
            .with_state(shared_state);

        let location = option_env!("TRAIN_SITE_LOCATION").unwrap_or("0.0.0.0:8080");
        let listener = tokio::net::TcpListener::bind(location).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(
                async {
                    let _ = tm_handle.await;
                }
                .into_future(),
            )
            .await
            .unwrap();
        println!("Axum Serve and Train Master had been shut down. Restarting in 3 secs...");
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
