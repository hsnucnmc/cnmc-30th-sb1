use axum::extract::State;
use axum::{extract::ws, routing::get, Router};

use train_backend::packet::*;

use ordered_float::OrderedFloat;

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
    state
        .view_request_tx
        .send((position, subscribe_request_tx))
        .await
        .unwrap();
    let mut update_receiver = match substribe_request_rx.await {
        Ok(rx) => rx,
        Err(_) => {
            println!("Failed to subscribe to train updates");
            socket
                .send(axum::extract::ws::Message::Close(Option::None))
                .await
                .unwrap();
            return;
        }
    };
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
                        println!("Received unexpected packet from client...");
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
    mut view_request_rx: tokio::sync::mpsc::Receiver<(
        TrainView,
        tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<ServerPacket>>,
    )>,
) {
    enum PositionObject {
        ViewLeftBound(u32),
        ViewRightBound(u32),
        TrackLeftEnd,
        TrackRightEnd,
    }

    println!("Server Started");

    let mut train_pos: OrderedFloat<f64> = 0f64.into();
    let mut going_right = true;
    let train_speed = 500f64; // 500 pixel per second
    let left_bound: OrderedFloat<f64> = 0f64.into();
    let right_bound: OrderedFloat<f64> = 4000f64.into();

    let mut train_pos_set = btreemultimap::BTreeMultiMap::new();
    let mut train_channels: std::collections::BTreeMap<
        u32,
        (tokio::sync::mpsc::Sender<ServerPacket>, OrderedFloat<f64>),
    > = std::collections::BTreeMap::new();

    train_pos_set.insert(left_bound, PositionObject::TrackLeftEnd);
    train_pos_set.insert(right_bound, PositionObject::TrackRightEnd);

    let mut next_viewer_id = 0;

    loop {
        println!("train_pos: {train_pos}, going: {going_right}");
        let wait_start = tokio::time::Instant::now();

        // calculate when will the train reach something
        let next_stop = *if going_right {
            let mut range = train_pos_set.range((
                std::ops::Bound::Excluded(&train_pos),
                std::ops::Bound::Included(&right_bound),
            ));
            range.next().unwrap() // We should always have at least one next thing in our range: the boundary object
        } else {
            let mut range = train_pos_set.range((
                std::ops::Bound::Included(&left_bound),
                std::ops::Bound::Excluded(&train_pos),
            ));
            range.next_back().unwrap()
        }
        .0;
        let wait_time = tokio::time::sleep(tokio::time::Duration::from_secs_f64(
            (*next_stop - *train_pos).abs() / train_speed,
        ));
        tokio::select! {
            biased;

            _ = wait_time => {
                let mut reached_end = false;
                for object in train_pos_set.get_vec(&next_stop).unwrap() { // We're guranteed to have at least one object at the stop
                        match object {
                            PositionObject::ViewLeftBound(id) => {
                                if going_right {
                                    // entering a left bound
                                    let (tx, length) = match train_channels.get(id) {
                                        Some(stuff) => stuff,
                                        None => {
                                            return;
                                        }
                                    }; // TODO DODODODODO

                                    let passing_time = length / train_speed;

                                    tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into()))
                                        .await
                                        .unwrap();
                                }
                            }
                            PositionObject::ViewRightBound(id) => {
                                if !going_right {
                                    // entering a right bound
                                    let (tx, length) = train_channels.get(id).unwrap();
                                    let passing_time = length / train_speed;
                                    tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into()))
                                        .await
                                        .unwrap();
                                }
                        }
                        PositionObject::TrackLeftEnd => {
                            reached_end = true;
                        }
                        PositionObject::TrackRightEnd => {
                            reached_end = true;
                        }
                    }
                }
                train_pos = next_stop;
                if reached_end {
                    going_right = !going_right;
                    // handle boundary cases
                    for object in train_pos_set.get_vec(&train_pos).unwrap() {
                        match object {
                            PositionObject::ViewLeftBound(id) => {
                                if going_right {
                                    // entering a left bound
                                    let (tx, length) = train_channels.get(id).unwrap();
                                    let passing_time = length / train_speed;
                                    tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into()))
                                        .await
                                        .unwrap();
                                }
                            }
                            PositionObject::ViewRightBound(id) => {
                                if !going_right {
                                    // entering a right bound
                                    let (tx, length) = train_channels.get(id).unwrap();
                                    let passing_time = length / train_speed;
                                    tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into()))
                                        .await
                                        .unwrap();
                                }
                        }
                        _ => {}
                        }
                    }
                }
            }

            request_result = view_request_rx.recv() => {
                // received new view request
                let (new_view, response_tx) = request_result.unwrap();
                let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(4);

                assert!(new_view.left<new_view.right);
                if new_view.left < left_bound || right_bound < new_view.right {
                    // invalid boundary for current track
                    continue;
                }

                response_tx.send(notify_rx).unwrap();

                let new_viewer_id = next_viewer_id;
                next_viewer_id += 1;
                train_channels.insert(new_viewer_id, (notify_tx, new_view.right-new_view.left));

                train_pos_set.insert(new_view.left, PositionObject::ViewLeftBound(new_viewer_id));
                train_pos_set.insert(new_view.right, PositionObject::ViewRightBound(new_viewer_id));

                if going_right {
                    train_pos = train_pos + (tokio::time::Instant::now() - wait_start).as_secs_f64() * train_speed;
                } else {
                    train_pos = train_pos - (tokio::time::Instant::now() - wait_start).as_secs_f64() * train_speed;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (view_request_tx, view_request_rx) = tokio::sync::mpsc::channel::<(
        TrainView,
        tokio::sync::oneshot::Sender<tokio::sync::mpsc::Receiver<ServerPacket>>,
    )>(32);

    tokio::spawn(async move { train_master(view_request_rx).await });

    // build our application with a single route

    let shared_state = AppState { view_request_tx };

    let assets_dir = std::path::PathBuf::from("../frontend/");

    let app: Router = Router::new()
        .fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
        ))
        .route("/ws", get(ws_get_handler))
        .with_state(shared_state);

    let location = option_env!("TRAIN_SITE_LOCATION").unwrap_or("0.0.0.0:8080");
    let listener = tokio::net::TcpListener::bind(location).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
