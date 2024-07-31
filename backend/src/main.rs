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

    let valid_train_id = (0..trains.len() as u32).collect();
    valid_id_tx.send(valid_train_id).unwrap();

    let tracks = vec![
        (
            TrackPiece {
                path: Bezier::Bezier3(Coord(100f64, 500f64), Coord(100f64, 100f64), Coord(500f64, 100f64)),
                color: "#66CCFF".into(),
                thickness: 5f64,
                length: 500f64,
            },
            0,
        ),
        (
            TrackPiece {
                path: Bezier::Bezier2(Coord(500f64, 100f64), Coord(900f64, 300f64)),
                color: "#66E5E5".into(),
                thickness: 5f64,
                length: 500f64,
            },
            1,
        ),
        (
            TrackPiece {
                path: Bezier::Bezier3(Coord(900f64, 300f64), Coord(900f64, 500f64), Coord(500f64, 500f64)),
                color: "#66FFCC".into(),
                thickness: 5f64,
                length: 500f64,
            },
            2,
        ),
        (
            TrackPiece {
                path: Bezier::Bezier4(Coord(500f64, 500f64),Coord(300f64, 300f64), Coord(100f64, 700f64), Coord(100f64, 500f64)),
                color: "#66E5E5".into(),
                thickness: 5f64,
                length: 500f64,
            },
            3,
        ),
    ];

    let mut viewer_channels = Vec::new();
    let (click_tx, mut click_rx) = tokio::sync::mpsc::channel(32);

    loop {
        let wait_start = tokio::time::Instant::now();

        // calculate when will the next train reach the end of it's track
        // let (next_stop, next_stop_trains) = {
        //     let mut next_stop = 
        //     for
        // }
        // let wait_time = tokio::time::sleep(tokio::time::Duration::from_secs_f64(
        //     (*next_stop - *train_pos).abs() / train_speed,
        // ));
        tokio::select! {
            biased;

            // _ = wait_time => {
            //     let mut reached_end = false;
            //     for object in train_pos_set.get_vec(&next_stop).unwrap() { // We're guranteed to have at least one object at the stop
            //             match object {
            //                 PositionObject::ViewLeftBound(id) => {
            //                     if going_right {
            //                         // entering a left bound
            //                         let (tx, length) = match viewer_channels.get(id) {
            //                             Some(stuff) => stuff,
            //                             None => {
            //                                 // Channel is already dead, we should also delete this entry.@
            //                                 // TODO
            //                                 continue;
            //                             }
            //                         };

            //                         let passing_time = length / train_speed;

            //                         // if tx failed sending, we delete the Channel
            //                         match tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into())).await {
            //                             Ok(_) => {},
            //                             Err(_) => {
            //                                 viewer_channels.remove(id);
            //                                 println!("Removed failed client handler from channel list");
            //                             },
            //                         }
            //                     }
            //                 }
            //                 PositionObject::ViewRightBound(id) => {
            //                     if !going_right {
            //                         // entering a right bound
            //                         let (tx, length) = match viewer_channels.get(id) {
            //                             Some(stuff) => stuff,
            //                             None => {
            //                                 // Channel is already dead, we should also delete this entry.
            //                                 // TODO
            //                                 continue;
            //                             }
            //                         };

            //                         let passing_time = length / train_speed;
            //                         match tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into())).await {
            //                             Ok(_) => {},
            //                             Err(_) => {
            //                                 viewer_channels.remove(id);
            //                                 println!("Removed failed client handler from channel list");
            //                             },
            //                         }
            //                     }
            //             }
            //             PositionObject::TrackLeftEnd => {
            //                 reached_end = true;
            //             }
            //             PositionObject::TrackRightEnd => {
            //                 reached_end = true;
            //             }
            //         }
            //     }
            //     train_pos = next_stop;
            //     if reached_end {
            //         going_right = !going_right;
            //         // handle boundary cases
            //         for object in train_pos_set.get_vec(&train_pos).unwrap() {
            //             match object {
            //                 PositionObject::ViewLeftBound(id) => {
            //                     if going_right {
            //                         // entering a left bound
            //                         let (tx, length) = match viewer_channels.get(id) {
            //                             Some(stuff) => stuff,
            //                             None => {
            //                                 // Channel is already dead, we should also delete this entry.
            //                                 // TODO
            //                                 continue;
            //                             }
            //                         };

            //                         let passing_time = length / train_speed;
            //                         match tx.send(ServerPacket::PacketRIGHT(*passing_time, 0f64, "train_right.png".into())).await {
            //                             Ok(_) => {},
            //                             Err(_) => {
            //                                 viewer_channels.remove(id);
            //                                 println!("Removed failed client handler from channel list");
            //                             },
            //                         }
            //                     }
            //                 }
            //                 PositionObject::ViewRightBound(id) => {
            //                     if !going_right {
            //                         // entering a right bound
            //                         let (tx, length) = match viewer_channels.get(id) {
            //                             Some(stuff) => stuff,
            //                             None => {
            //                                 // Channel is already dead, we should also delete this entry.
            //                                 // TODO
            //                                 continue;
            //                             }
            //                         };

            //                         let passing_time = length / train_speed;
            //                         match tx.send(ServerPacket::PacketLEFT(*passing_time, 0f64, "train_left.png".into())).await {
            //                             Ok(_) => {},
            //                             Err(_) => {
            //                                 viewer_channels.remove(id);
            //                                 println!("Removed failed client handler from channel list");
            //                             },
            //                         }
            //                     }
            //             }
            //             _ => {}
            //             }
            //         }
            //     }
            // }
            clicked = click_rx.recv() => {
                let clicked = clicked.unwrap();
                println!("Train#{} is clicked", clicked);
            }

            request_result = view_request_rx.recv() => {
                // received new view request
                let response_tx = request_result.unwrap();
                let (notify_tx, notify_rx) = tokio::sync::mpsc::channel(4);
                
                response_tx.send((notify_rx, click_tx.clone())).unwrap();
                notify_tx.send(ServerPacket::PacketTRACK(tracks.iter().map(
                    |a| (a.1, a.0.path, a.0.color.clone(), a.0.thickness)
                ).collect())).await.unwrap();
                notify_tx.send(ServerPacket::PacketTRAIN(0, 0, 0f64, tokio::time::Duration::from_millis(10000), "train_right_debug.png".into())).await.unwrap();
                notify_tx.send(ServerPacket::PacketTRAIN(1, 1, 0f64, tokio::time::Duration::from_millis(20000), "train_right_debug.png".into())).await.unwrap();
                notify_tx.send(ServerPacket::PacketTRAIN(2, 2, 0f64, tokio::time::Duration::from_millis(15000), "train_right_debug.png".into())).await.unwrap();
                notify_tx.send(ServerPacket::PacketTRAIN(3, 3, 0f64, tokio::time::Duration::from_millis(8000), "train_right_debug.png".into())).await.unwrap();
                notify_tx.send(ServerPacket::PacketTRAIN(4, 3, 0f64, tokio::time::Duration::from_millis(12000), "train_left_debug.png".into())).await.unwrap();
                viewer_channels.push(notify_tx);
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
