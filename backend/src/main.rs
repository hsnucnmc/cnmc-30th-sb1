use axum::extract::State;
use axum::{extract::ws, Router, routing::get};

use train_backend::packet::*;

#[derive(Clone)]
struct AppState {
    view_request_tx: tokio::sync::mpsc::Sender<(
        TrainView,
        tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<ServerPacket>>,
    )>,
}

async fn ws_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ws_client_handler(socket, state))
}

async fn ws_client_handler(mut socket: ws::WebSocket, state: AppState) {
    println!("New websocket connection has established...");

    let response = match tokio::time::timeout(tokio::time::Duration::from_secs(10), socket.recv())
        .await
    {
        Err(_) => {
            println!("A websocket connection took too long to send a POSITION...");
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
        Ok(None) => {
            println!("A websocket connection abruptly closed before sending a OLLEH response...");
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
        Ok(Some(Err(_))) => {
            println!("A websocket connection caused a error before sending a OLLEH response...");
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
        Ok(Some(Ok(response))) => response,
    };

    let text_response = match response {
        ws::Message::Close(_) => {
            println!("A websocket connection closed before sending a response...");
            return;
        }
        ws::Message::Text(text_response) => text_response,
        _ => {
            println!("A websocket connection sent a response that's not a text message...");
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
    };

    let position = match text_response.parse::<ClientPacket>() {
        Err(err) => {
            println!("A websocket connection sent a packet expected to be a POSITION but failed parsing:\n\t{}", err);
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
        Ok(ClientPacket::PacketPOSITION(position)) => position,
    };

    let (subscribe_request_tx, substribe_request_rx) = tokio::sync::oneshot::channel();
    state.view_request_tx.send((position, subscribe_request_tx)).await.unwrap();
    let mut update_receiver = substribe_request_rx.await.unwrap();
    loop {
        tokio::select! {
            biased;

            packet = socket.recv() => {
                let packet = packet.unwrap();
                let _ = match packet {
                    Err(_) => {
                        println!("A websocket connection produced a error (probably abruptly closed)...");
                        break;
                    }
                    // Ok(axum::extract::ws::Message::Close(_)) => {
                    //     println!("Client leaved...");
                    //     break;
                    // }
                    // Ok(axum::extract::ws::Message::Text(text)) => text,
                    Ok(_) => {
                        println!("Received unexpected non-text packet from client...");
                        break;
                    }
                };

                // let packet = match packet.parse::<ClientPacket>() {
                //     Err(err) => {
                //         println!("A websocket connection sent a packet expected to be a MOVE but failed parsing:\n\t{}", err);
                //         break;
                //     }
                //     Ok(packet) => packet,
                // };

                // match packet {
                //     ClientPacket::PacketOLLEH(_) => {
                //         println!("A websocket connection sent a packet expected to be a MOVE but is a OLLEH");
                //         break;
                //     }
                //     ClientPacket::PacketMOVE(index, action) => {
                //         if index >= bomb_count {
                //             println!("A websocket connection sent a MOVE packet with a index out of bound");
                //             break;
                //         }
                //         match &bomb_actions[index as usize] {
                //             None => {
                //                 println!("A websocket connection sent a MOVE packet while not holding the specified bomb");
                //                 break;
                //             }
                //             _ => {}
                //         }
                //         bomb_actions[index as usize].take().unwrap().send(Ok(action)).unwrap();
                //         bomb_counter[index as usize]+=1;
                //     }
                // }
            }

            update = update_receiver.recv() => {
                socket.send(update.unwrap().into()).await.unwrap();
            }
        };
    }

    // this part SHOULD be optional after the problem is fixed
    // for mut action_tx in bomb_actions {
    // if let Some(action_tx) = action_tx.take() {
    // action_tx.send(BombMoveAction::R1).unwrap();
    // }
    // }

    let _ = socket
        .send(axum::extract::ws::Message::Close(Option::None))
        .await;
    return;
}

async fn train_master(
    view_request_rx: tokio::sync::mpsc::Receiver<(
        TrainView,
        tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<ServerPacket>>,
    )>,
) {
    let train_pos = 0f64;
    
}

#[tokio::main]
async fn main() {
    let (view_request_tx, view_request_rx) = tokio::sync::mpsc::channel::<(
        TrainView,
        tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<ServerPacket>>,
    )>(32);

    tokio::spawn(async move { train_master(view_request_rx).await });

    // build our application with a single route

    let shared_state = AppState {
        view_request_tx,
    };

    let assets_dir = std::path::PathBuf::from("../frontend/");

    let app: Router = Router::new()
        .fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
        ))
        .route("/ws", get(ws_get_handler))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
