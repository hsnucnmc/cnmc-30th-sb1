use std::collections::{BTreeMap, BTreeSet, HashSet};

use axum::extract::State;
use axum::{extract::ws, routing::get, Router};

use tokio::sync::{mpsc, oneshot, watch};

use train_backend::packet::*;

#[derive(Clone)]
struct AppState {
    view_request_tx:
        mpsc::Sender<oneshot::Sender<(mpsc::Receiver<ServerPacket>, mpsc::Sender<TrainID>)>>,
    valid_id: watch::Receiver<BTreeSet<TrainID>>,
    derail_tx: mpsc::Sender<()>,
}

async fn ws_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ws_client_handler(socket, state))
}

async fn derail_handler(State(state): State<AppState>) {
    let _ = state.derail_tx.send(()).await;
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

    let _ = socket.send(ws::Message::Close(Option::None)).await;
    return;
}

async fn train_master(
    mut view_request_rx: mpsc::Receiver<
        oneshot::Sender<(mpsc::Receiver<ServerPacket>, mpsc::Sender<TrainID>)>,
    >,
    valid_id_tx: watch::Sender<BTreeSet<TrainID>>,
    mut derail_rx: mpsc::Receiver<()>,
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

    impl TrainInstance {
        fn to_packet(&self, id: u32, tracks: &BTreeMap<u32, TrackPiece>) -> ServerPacket {
            ServerPacket::PacketTRAIN(
                id,
                self.current_track,
                self.progress,
                tokio::time::Duration::from_secs_f64(
                    tracks.get(&self.current_track).unwrap().length / self.properties.speed,
                ),
                self.properties.image.clone(),
            )
        }
    }

    println!("Server Started");

    let mut trains = vec![
        TrainInstance {
            properties: TrainProperties {
                speed: 500f64,
                image: "train_right_debug.png".into(),
            },
            current_track: 0,
            progress: 0.0,
        },
        TrainInstance {
            properties: TrainProperties {
                speed: 250f64,
                image: "train_right_debug.png".into(),
            },
            current_track: 0,
            progress: 0.0,
        },
    ];

    let valid_train_id = (0..trains.len() as u32).collect();
    valid_id_tx.send(valid_train_id).unwrap();

    let tracks = {
        let tracks_vec = vec![
            // 1
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(2000f64, 100f64),
                    Coord(2200f64, 400f64),
                    Coord(2900f64, 200f64),
                    Coord(2800f64, 500f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            //2
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(2800f64, 500f64),
                    Coord(2400f64, 300f64),
                    Coord(2400f64, 550f64),
                    Coord(2200f64, 550f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,            
            },    
            // 3
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(2200f64, 550f64),
                    Coord(2000f64, 550f64),
                    Coord(2100f64, 450f64),
                    Coord(1800f64, 350f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 4
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(1800f64, 350f64),
                    Coord(1300f64, 400f64),                    
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 5
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(1300f64, 400f64),
                    Coord(1200f64, 550f64),
                    Coord(1000f64, 400f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 6
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(1000f64, 400f64),
                    Coord(650f64, 300f64),
                    Coord(300f64, 400f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 7
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(300f64, 400f64),
                    Coord(175f64, 550f64),
                    Coord(-200f64, 550f64),
                    Coord(-200f64, 300f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 8
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(-200f64, 300f64),
                    Coord(-445f64, 500f64),
                    Coord(-1175f64, 550f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 9
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(-1175f64, 550f64),
                    Coord(-1500f64, 400f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 10
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(-1500f64, 400f64),
                    Coord(-2150f64, 450f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 11
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(-2150f64, 450f64),
                    Coord(-2600f64, 550f64),
                    Coord(-2800f64, 100f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 12
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(-2800f64, 100f64),
                    Coord(-2100f64, 100f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 13
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-2100f64, 100f64),
                    Coord(-1900f64, 150f64),
                    Coord(-2000f64, 300f64),
                    Coord(-1800f64, 350f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 14
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-1800f64, 350f64),
                    Coord(-1700f64, 350f64),
                    Coord(-1700f64, 300f64),
                    Coord(-1700f64, 100f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 15
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-1700f64, 100f64),
                    Coord(-1500f64, 100f64),
                    Coord(-1600f64, 300f64),
                    Coord(-1200f64, 300f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 16
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-1200f64, 300f64),
                    Coord(-1100f64, 300f64),
                    Coord(-950f64, 200f64),
                    Coord(-900f64, 100f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 17
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-900f64, 100f64),
                    Coord(-800f64, 100f64),
                    Coord(-700f64, 150f64),
                    Coord(-400f64, 200f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 18
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(-400f64, 200f64),
                    Coord(0f64, 200f64),
                    Coord(0f64, 50f64),
                    Coord(400f64, 200f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 19
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(400f64, 200f64),
                    Coord(750f64, 200f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 20
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(750f64, 200f64),
                    Coord(800f64, 300f64),
                    Coord(900f64, 200f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 21
            TrackPiece {
                path: Bezier::Bezier4(
                    Coord(900f64, 200f64),
                    Coord(1100f64, 100f64),
                    Coord(1100f64, 300f64),
                    Coord(1300f64, 300f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 22
            TrackPiece {
                path: Bezier::Bezier2(
                    Coord(1300f64, 300f64),
                    Coord(1700f64, 200f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },
            // 23
            TrackPiece {
                path: Bezier::Bezier3(
                    Coord(1700f64, 200f64),
                    Coord(1900f64, 250f64),
                    Coord(2000f64, 100f64),
                ),
                color: "#66FFCC".into(),
                thickness: 20f64,
                length: 500f64,
            },

                        

        ];
        let mut tracks = BTreeMap::new();
        for (i, track) in tracks_vec.into_iter().enumerate() {
            tracks.insert(i as u32, track);
        }
        tracks
    };

    let mut viewer_channels: BTreeMap<u32, mpsc::Sender<ServerPacket>> = BTreeMap::new();
    let mut next_viewer_serial = 0u32;
    let (click_tx, mut click_rx) = mpsc::channel(32);

    loop {
        let wait_start = tokio::time::Instant::now();

        // calculate when will the next train reach the end of it's current track
        let wait_time = trains
            .iter()
            .map(|train| {
                ordered_float::OrderedFloat(
                    tracks.get(&train.current_track).unwrap().length * (1f64 - train.progress)
                        / train.properties.speed,
                )
            })
            .min()
            .unwrap();

        let wait_time = tokio::time::sleep(tokio::time::Duration::from_secs_f64(wait_time.0));
        tokio::select! {
            biased;

            _ = wait_time => {
                let wait_end = tokio::time::Instant::now();
                for (i, train) in trains.iter_mut().enumerate() {
                    train.progress += ((wait_end - wait_start).as_secs_f64() * train.properties.speed) / tracks.get(&train.current_track).unwrap().length;
                    if train.progress >= 1f64 {
                        train.current_track += 1;
                        if train.current_track >= tracks.len() as u32 {
                            train.current_track = 0;
                        }
                        train.progress = 0f64; // TODO: actually calculate progress

                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(i as u32, &tracks)).await;
                        }
                    }
                }
            }
            clicked = click_rx.recv() => {
                let clicked = clicked.unwrap();
                println!("Train#{} is clicked", clicked);

                let wait_end = tokio::time::Instant::now();
                for (i, train) in trains.iter_mut().enumerate() {
                    train.progress += ((wait_end - wait_start).as_secs_f64() * train.properties.speed) / tracks.get(&train.current_track).unwrap().length;
                    if train.progress >= 1f64 {
                        train.current_track += 1;
                        if train.current_track >= tracks.len() as u32 {
                            train.current_track = 0;
                        }
                        train.progress = 0f64; // TODO: actually calculate progress

                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(i as u32, &tracks)).await;
                        }
                    }
                }
            }

            request_result = view_request_rx.recv() => {
                // received new view request
                let response_tx = request_result.unwrap();
                let (notify_tx, notify_rx) = mpsc::channel(4);

                response_tx.send((notify_rx, click_tx.clone())).unwrap();
                notify_tx.send(ServerPacket::PacketTRACK(tracks.iter().map(
                    |a| (*a.0, a.1.path, a.1.color.clone(), a.1.thickness)
                ).collect())).await.unwrap();

                let wait_end = tokio::time::Instant::now();
                for (i, train) in trains.iter_mut().enumerate() {
                    train.progress += ((wait_end - wait_start).as_secs_f64() * train.properties.speed) / tracks.get(&train.current_track).unwrap().length;
                    if train.progress >= 1f64 {
                        train.current_track += 1;
                        if train.current_track >= tracks.len() as u32 {
                            train.current_track = 0;
                        }
                        train.progress = 0f64; // TODO: actually calculate progress

                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(i as u32, &tracks)).await;
                        }
                    }
                    notify_tx.send(train.to_packet(i as u32, &tracks)).await;
                }
                viewer_channels.insert(next_viewer_serial, notify_tx);
                next_viewer_serial += 1;
            }

            _ = derail_rx.recv() => {
                println!("RECEIVED DERAIL REQUEST!!!");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (view_request_tx, view_request_rx) = mpsc::channel(32);

    let (valid_id_tx, valid_id_rx) = watch::channel(BTreeSet::new());

    let (derail_tx, derail_rx) = mpsc::channel(1);

    tokio::spawn(async move { train_master(view_request_rx, valid_id_tx, derail_rx).await });

    // build our application with a single route

    let shared_state = AppState {
        view_request_tx,
        valid_id: valid_id_rx,
        derail_tx,
    };

    let assets_dir = std::path::PathBuf::from("../frontend/");

    let app: Router = Router::new()
        .fallback_service(axum::routing::get_service(
            tower_http::services::ServeDir::new(assets_dir).append_index_html_on_directories(true),
        ))
        .route(
            "/derailer",
            axum::routing::get_service(tower_http::services::ServeFile::new(
                "../frontend/derailer.html",
            )),
        )
        .route("/ws", get(ws_get_handler))
        .route("/force-derail", get(derail_handler))
        .with_state(shared_state);

    let location = option_env!("TRAIN_SITE_LOCATION").unwrap_or("0.0.0.0:8080");
    let listener = tokio::net::TcpListener::bind(location).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
