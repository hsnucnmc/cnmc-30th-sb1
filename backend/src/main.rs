use axum::extract::State;
use axum::{extract::ws, routing::get, Router};

use train_backend::packet::*;

use ordered_float::OrderedFloat;

#[derive(Clone)]
struct AppState {
    view_request_tx: tokio::sync::mpsc::Sender<
        tokio::sync::oneshot::Sender<(
            tokio::sync::mpsc::Receiver<ServerPacket>,
            tokio::sync::mpsc::Sender<TrainID>,
        )>,
    >,
    valid_id: tokio::sync::watch::Receiver<std::collections::BTreeSet<TrainID>>,
}

async fn ws_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ws_client_handler(socket, state))
}

async fn ws_client_handler(mut socket: ws::WebSocket, state: AppState) {
    println!("New websocket connection has established...");

    let (subscribe_request_tx, substribe_request_rx) = tokio::sync::oneshot::channel();
    state
        .view_request_tx
        .send(subscribe_request_tx)
        .await
        .unwrap();

    let (mut update_receiver, click_sender) = match substribe_request_rx.await {
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
                let packet = match packet {
                    Err(_) => {
                        println!("A websocket connection produced a error (probably abruptly closed)...");
                        break;
                    }
                    Ok(ws::Message::Text(packet)) => packet,
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
                    ClientPacket::PacketCLICK(train_id) => {
                        if !state.valid_id.borrow().contains(&train_id) {
                            println!("A websocket connection sent a packet expected to be a CLICK but contains invalid train id");
                            break;
                        } else {
                            match click_sender.send(train_id).await {
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
                        println!("Failed to subscribe to train updates");
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

    let _ = socket
        .send(axum::extract::ws::Message::Close(Option::None))
        .await;
    return;
}

async fn train_master(
    mut view_request_rx: tokio::sync::mpsc::Receiver<
        tokio::sync::oneshot::Sender<(
            tokio::sync::mpsc::Receiver<ServerPacket>,
            tokio::sync::mpsc::Sender<TrainID>,
        )>,
    >,
    mut valid_id_tx: tokio::sync::watch::Sender<std::collections::BTreeSet<TrainID>>,
) {
    struct TrackPiece {
        path: Bezier,         // px
        color: Color,         // #FFFFFF
        thickness: Thickness, // px
        length: f64,          // px
    }

    struct TrainProperties {
        speed: f64, // px/s
        image: String,
        // image_left: String,
        // image_right: String,
    }

    struct TrainInstance {
        properties: TrainProperties,
        current_track: u32,
        progress: f64, // 0 ~ 1
    }

    println!("Server Started");

    let trains = vec![
        TrainInstance {
            properties: TrainProperties {
                speed: 500f64,
                image: "train_right.png".into(),
            },
            current_track: 0,
            progress: 0.0,
        },
        TrainInstance {
            properties: TrainProperties {
                speed: 250f64,
                image: "train_2_right.png".into(),
            },
            current_track: 1,
            progress: 0.0,
        },
    ];

    let tracks = vec![
        (
            TrackPiece {
                path: Bezier::Bezier3(Coord(100f64, 500f64), Coord(100f64, 100f64), Coord(500f64, 100f64)),
                color: "#66CCFF".into(),
                thickness: 1f64,
                length: 500f64,
            },
            1,
        ),
        (
            TrackPiece {
                path: Bezier::Bezier3(Coord(500f64, 100f64), Coord(500f64, 500f64), Coord(100f64, 500f64)),
                color: "#66FFCC".into(),
                thickness: 1f64,
                length: 500f64,
            },
            1,
        ),
    ];

    let mut train_channels: std::collections::BTreeMap<
        u32,
        (tokio::sync::mpsc::Sender<ServerPacket>, OrderedFloat<f64>),
    > = std::collections::BTreeMap::new();

    let mut next_viewer_id = 0;

    loop {
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
                                            // Channel is already dead, we should also delete this entry.@
                                            // TODO
                                            continue;
                                        }
                                    };

                                    let passing_time = length / train_speed;

                                    // if tx failed sending, we delete the Channel
                                    match tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into())).await {
                                        Ok(_) => {},
                                        Err(_) => {
                                            train_channels.remove(id);
                                            println!("Removed failed client handler from channel list");
                                        },
                                    }
                                }
                            }
                            PositionObject::ViewRightBound(id) => {
                                if !going_right {
                                    // entering a right bound
                                    let (tx, length) = match train_channels.get(id) {
                                        Some(stuff) => stuff,
                                        None => {
                                            // Channel is already dead, we should also delete this entry.
                                            // TODO
                                            continue;
                                        }
                                    };

                                    let passing_time = length / train_speed;
                                    match tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into())).await {
                                        Ok(_) => {},
                                        Err(_) => {
                                            train_channels.remove(id);
                                            println!("Removed failed client handler from channel list");
                                        },
                                    }
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
                                    let (tx, length) = match train_channels.get(id) {
                                        Some(stuff) => stuff,
                                        None => {
                                            // Channel is already dead, we should also delete this entry.
                                            // TODO
                                            continue;
                                        }
                                    };

                                    let passing_time = length / train_speed;
                                    match tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into())).await {
                                        Ok(_) => {},
                                        Err(_) => {
                                            train_channels.remove(id);
                                            println!("Removed failed client handler from channel list");
                                        },
                                    }
                                }
                            }
                            PositionObject::ViewRightBound(id) => {
                                if !going_right {
                                    // entering a right bound
                                    let (tx, length) = match train_channels.get(id) {
                                        Some(stuff) => stuff,
                                        None => {
                                            // Channel is already dead, we should also delete this entry.
                                            // TODO
                                            continue;
                                        }
                                    };

                                    let passing_time = length / train_speed;
                                    match tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into())).await {
                                        Ok(_) => {},
                                        Err(_) => {
                                            train_channels.remove(id);
                                            println!("Removed failed client handler from channel list");
                                        },
                                    }
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
    let (view_request_tx, view_request_rx) = tokio::sync::mpsc::channel(32);

    let (valid_id_tx, valid_id_rx) = tokio::sync::watch::channel(std::collections::BTreeSet::new());

    tokio::spawn(async move { train_master(view_request_rx, valid_id_tx).await });

    // build our application with a single route

    let shared_state = AppState {
        view_request_tx,
        valid_id: valid_id_rx,
    };

    let assets_dir = std::path::PathBuf::from("../frontend/");

    let app: Router = Router::new()
        .fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
        ))
        .route("/ws", get(ws_get_handler))
        // .route("/force-derail", get(derail_handler))
        .with_state(shared_state);

    let location = option_env!("TRAIN_SITE_LOCATION").unwrap_or("0.0.0.0:8080");
    let listener = tokio::net::TcpListener::bind(location).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
