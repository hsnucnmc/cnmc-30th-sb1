use std::collections::BTreeSet;

use crate::{routing::RoutingInfo, AppState};
use axum::{
    extract::{ws, Path, State}, response::IntoResponse, Json
};
use packet::*;
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

pub async fn derail_handler(
    State(state): State<AppState>,
    Path(track_name): Path<String>,
) -> axum::http::StatusCode {
    if !(track_name == "") {
        if !track_name
            .chars()
            .all(|chr: char| chr.is_alphanumeric() || chr == '_' || chr == '-')
        {
            return axum::http::StatusCode::BAD_REQUEST;
        }
        let existing: BTreeSet<String> = match serde_json::from_str(
            &std::fs::read_to_string("tracks/existing.json").unwrap_or("[]".into()),
        ) {
            Ok(set) => set,
            Err(_) => {
                println!("Failed parsing exsiting.json");
                return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
            }
        };
        if !existing.contains(&track_name) {
            return axum::http::StatusCode::BAD_REQUEST;
        }
    }

    *state.next_track.lock().await = Some(track_name);
    let _ = state.derail_tx.send(()).await;
    axum::http::StatusCode::OK
}

pub async fn list_track_handler() -> axum::response::Response {
    Json(
        match serde_json::from_str::<BTreeSet<String>>(
            &std::fs::read_to_string("tracks/existing.json").unwrap_or("[]".into()),
        ) {
            Ok(stuff) => stuff,
            Err(_) => {
                println!("Failed parsing exsiting.json");
                return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        },
    )
    .into_response()
}

pub async fn list_nodes_handler(State(state): State<AppState>) -> axum::response::Response {
    let (sender, receiver) = oneshot::channel();
    match state.list_nodes_request.send(sender).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed receiving node list");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(match receiver.await {
        Ok(list) => list,
        Err(_) => {
            println!("Failed receiving node list");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    })
    .into_response()
}

pub async fn node_type_handler(
    State(state): State<AppState>,
    Path(node_id): Path<NodeID>,
) -> axum::response::Response {
    let (sender, receiver) = oneshot::channel();
    match state.node_type_request.send((node_id, sender)).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed receiving node type");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(match receiver.await {
        Ok(Some(node_type)) => node_type.to_string(),
        Ok(None) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => {
            println!("Failed receiving node type");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    })
    .into_response()
}

pub async fn node_get_routing_handler(
    State(state): State<AppState>,
    Path(node_id): Path<NodeID>,
) -> axum::response::Response {
    let (sender, receiver) = oneshot::channel();
    match state.node_get_routing_request.send((node_id, sender)).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed receiving node routing info");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    Json(match receiver.await {
        Ok(Some(routing_info)) => {
            println!("{:?}", serde_json::to_string(&routing_info));
            routing_info
        },
        Ok(None) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => {
            println!("Failed receiving node routing info");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    })
    .into_response()
}

pub async fn node_set_routing_handler(
    State(state): State<AppState>,
    Path(node_id): Path<NodeID>,
    Json(routing): Json<RoutingInfo>,
) -> axum::response::Response {
    if let Err(err) = routing.check() {
        let mut response = axum::response::Response::default();
        *response.body_mut() = err.to_string().into_bytes().into();
        *response.status_mut() = axum::http::StatusCode::UNPROCESSABLE_ENTITY;
        return response;
    }

    let (sender, receiver) = oneshot::channel();
    match state.node_get_routing_request.send((node_id, sender)).await {
        Ok(_) => {}
        Err(_) => {
            println!("Failed receiving current node routing info");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match receiver.await {
        Ok(Some(_)) => {},
        Ok(None) => {
            return axum::http::StatusCode::NOT_FOUND.into_response();
        }
        Err(_) => {
            println!("Failed receiving node routing info");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    match state
        .node_set_routing_request
        .send((node_id, routing))
        .await
    {
        Ok(_) => axum::http::StatusCode::OK.into_response(),
        Err(_) => {
            println!("Failed requesting node routing change");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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

    let (mut update_receiver, click_sender, switch_sender) = match substribe_request_rx.await {
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
                    ClientPacket::PacketSWITCH(node_id, modifier) => {
                        match switch_sender.send((node_id, modifier)).await {
                            Ok(_) => (),
                            Err(_) => {
                                println!("Failed sending switch updates to train master");
                                break;
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
