use std::collections::BTreeSet;

use crate::AppState;
use packet::*;
use axum::extract::{ws, State};
use tokio::sync::oneshot;

pub async fn ws_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ws_client_handler(socket, state))
}

pub async fn ctrl_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ctrl_client_handler(socket, state))
}

pub async fn derail_handler(State(state): State<AppState>) {
    let _ = state.derail_tx.send(()).await;
}

pub async fn list_track_handler() -> axum::Json<BTreeSet<String>> {
    axum::Json(serde_json::from_str(
        &std::fs::read_to_string("tracks/existing.json").unwrap_or("[]".into()),
    ).unwrap())
}

async fn ws_client_handler(mut socket: ws::WebSocket, state: AppState) {
    println!("New websocket connection has established...");

    let (subscribe_request_tx, substribe_request_rx) = oneshot::channel();
    match state.view_request_tx.send(subscribe_request_tx).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to send update subscription, is train master dead?");
            let _ = socket.send(ws::Message::Close(Option::None)).await;
            return;
        }
    };

    let (mut update_receiver, click_sender) = match substribe_request_rx.await {
        Ok(rx) => rx,
        Err(_) => {
            println!("Failed to subscribe to train updates");
            let _ = socket.send(ws::Message::Close(Option::None)).await;
            return;
        }
    };

    loop {
        tokio::select! {
            biased;

            packet = socket.recv() => {
                let packet = packet.unwrap();
                let packet = match packet {
                    Err(_) => {
                        println!("A websocket connection produced a error (probably abruptly closed)...");
                        break;
                    }
                    Ok(ws::Message::Text(packet)) => packet,
                    Ok(ws::Message::Close(_)) => {
                        println!("A websocket connection sent a close packet...");
                        break;
                    }
                    Ok(_) => {
                        println!("A websocket connection sent a packet with an unexpected type...");
                        break;
                    }
                };

                let packet = match packet.parse::<ClientPacket>() {
                    Err(err) => {
                        println!("A websocket connection sent a packet expected to be a CLICK but failed parsing:\n\t{}", err);
                        break;
                    }
                    Ok(packet) => packet,
                };

                match packet {
                    ClientPacket::PacketCLICK(train_id, modifier) => {
                        if !state.valid_id.borrow().contains(&train_id) {
                            println!("A websocket connection sent a packet expected to be a CLICK but contains invalid train id");
                            break;
                        } else {
                            match click_sender.send((train_id, modifier)).await {
                                Ok(_) => (),
                                Err(_) => {
                                    println!("Failed sending click updates to train master");
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            update = update_receiver.recv() => {
                match update {
                    Some(update) => {
                        socket.send(update.into()).await.unwrap();
                    }
                    None => {
                        println!("Failed to receive train updates");
                        break;
                    }
                }

            }
        };
    }

    // this part SHOULD be optional after the problem is fixed
    // for mut action_tx in bomb_actions {
    // if let Some(action_tx) = action_tx.take() {
    // action_tx.send(BombMoveAction::R1).unwrap();
    // }
    // }

    let _ = socket.send(ws::Message::Close(Option::None)).await;
    return;
}

async fn ctrl_client_handler(mut socket: ws::WebSocket, state: AppState) {
    println!("New control connection has established...");

    let (subscribe_request_tx, subscribe_request_rx) = oneshot::channel();
    match state.ctrl_request_tx.send(subscribe_request_tx).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed to request sending controling request, is train master dead?");
            let _ = socket.send(ws::Message::Close(Option::None)).await;
            return;
        }
    };

    let ctrl_sender = match subscribe_request_rx.await {
        Ok(rx) => rx,
        Err(_) => {
            println!("Failed to start sending control requests...");
            let _ = socket.send(ws::Message::Close(Option::None)).await;
            return;
        }
    };

    loop {
        let packet = socket.recv().await;
        let packet = packet.unwrap();
        let packet = match packet {
            Err(_) => {
                println!("A control connection produced a error (probably abruptly closed)...");
                break;
            }
            Ok(ws::Message::Text(packet)) => packet,
            Ok(ws::Message::Close(_)) => {
                println!("A control connection sent a close packet...");
                break;
            }
            Ok(_) => {
                println!("A control connection sent a packet with an unexpected type...");
                break;
            }
        };

        let packet = match packet.parse::<CtrlPacket>() {
            Err(err) => {
                println!(
                    "A control connection sent a packet but failed parsing:\n\t{}",
                    err
                );
                break;
            }
            Ok(packet) => packet,
        };

        match ctrl_sender.send(packet).await {
            Err(_) => {
                println!(
                    "A control connection sent a packet but failed relaying to train master..."
                );
                break;
            }
            Ok(_) => {}
        };
    }

    let _ = socket.send(ws::Message::Close(Option::None)).await;
    return;
}
