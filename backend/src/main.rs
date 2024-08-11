use std::collections::{BTreeMap, BTreeSet};
use std::future::IntoFuture;

use axum::extract::State;
use axum::{extract::ws, routing::get, Router};

use rand::{thread_rng, Rng};
use tokio::sync::{mpsc, oneshot, watch};

use train_backend::packet::*;

#[derive(Clone)]
struct AppState {
    view_request_tx: mpsc::Sender<
        oneshot::Sender<(
            mpsc::Receiver<ServerPacket>,
            mpsc::Sender<(TrainID, ClickModifier)>,
        )>,
    >,
    ctrl_request_tx: mpsc::Sender<oneshot::Sender<mpsc::Sender<CtrlPacket>>>,
    valid_id: watch::Receiver<BTreeSet<TrainID>>,
    derail_tx: mpsc::Sender<()>,
}

async fn ws_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ws_client_handler(socket, state))
}

async fn ctrl_get_handler(
    ws: ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| ctrl_client_handler(socket, state))
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

async fn train_master(
    mut view_request_rx: mpsc::Receiver<
        oneshot::Sender<(
            mpsc::Receiver<ServerPacket>,
            mpsc::Sender<(TrainID, ClickModifier)>,
        )>,
    >,
    mut ctrl_request_rx: mpsc::Receiver<oneshot::Sender<mpsc::Sender<CtrlPacket>>>,
    valid_id_tx: watch::Sender<BTreeSet<TrainID>>,
    mut derail_rx: mpsc::Receiver<()>,
) {
    struct Node {
        id: NodeID,
        coord: Coord,
        connections: BTreeMap<TrackID, Direction>, // 順向還是反向進入接點；
    }

    impl Node {
        fn to_packet(&self) -> ServerPacket {
            ServerPacket::PacketNODE(self.id, self.coord)
        }
    }

    struct TrackPiece {
        id: TrackID,
        start: NodeID,
        end: NodeID,
        path: Bezier,         // px
        color: Color,         // #FFFFFF
        thickness: Thickness, // px
        length: f64,          // px
    }

    impl TrackPiece {
        fn new(
            id: TrackID,
            start_id: NodeID,
            end_id: NodeID,
            nodes: &mut BTreeMap<NodeID, Node>,
            diff: BezierDiff,
            color: Color,
            thickness: Thickness,
        ) -> TrackPiece {
            nodes
                .get_mut(&start_id)
                .unwrap()
                .connections
                .insert(id, Direction::Backward);
            nodes
                .get_mut(&end_id)
                .unwrap()
                .connections
                .insert(id, Direction::Forward);

            let path = Bezier::new(
                nodes.get(&start_id).unwrap().coord,
                nodes.get(&end_id).unwrap().coord,
                diff,
            );

            TrackPiece {
                id,
                start: start_id,
                end: end_id,
                path,
                color,
                thickness,
                length: path.fast_length(),
            }
        }
    }

    struct TrainProperties {
        speed: f64, // px/s
        image_forward: String,
        image_backward: String,
    }

    struct TrainInstance {
        properties: TrainProperties,
        current_track: u32,
        progress: f64,        // 0 ~ 1
        direction: Direction, // backward direction: progress goes from 1 to 0
    }

    impl TrainInstance {
        fn estimated_time_left(&self, tracks: &BTreeMap<u32, TrackPiece>) -> Duration {
            Duration::from_secs_f64(
                match self.direction {
                    Direction::Forward => 1f64 - self.progress,
                    Direction::Backward => self.progress,
                } * tracks.get(&self.current_track).unwrap().length
                    / self.properties.speed,
            )
        }

        // update train after a ceratin duration of movement, return true when train has switched to another track
        fn move_with_time(
            &mut self,
            duration: Duration,
            nodes: &BTreeMap<NodeID, Node>,
            tracks: &BTreeMap<TrackID, TrackPiece>,
        ) -> bool {
            let train = self;
            let mut flag = false;
            let mut move_distance = duration.as_secs_f64() * train.properties.speed;

            loop {
                let required_distance = match train.direction {
                    Direction::Forward => 1f64 - train.progress,
                    Direction::Backward => train.progress,
                } * tracks.get(&train.current_track).unwrap().length;

                if required_distance <= move_distance {
                    move_distance -= required_distance;
                    let end_node = match train.direction {
                        Direction::Forward => {
                            let end_node = nodes
                                .get(&tracks.get(&train.current_track).unwrap().end)
                                .unwrap();
                            assert!(
                                end_node.connections.get(&train.current_track).unwrap()
                                    == &Direction::Forward
                            );
                            end_node
                        }
                        Direction::Backward => {
                            let end_node = nodes
                                .get(&tracks.get(&train.current_track).unwrap().start)
                                .unwrap();
                            assert!(
                                end_node.connections.get(&train.current_track).unwrap()
                                    == &Direction::Backward
                            );
                            end_node
                        }
                    };
                    let next_track = loop {
                        let nth = thread_rng().gen_range(0..end_node.connections.len());
                        let next_track = end_node.connections.iter().nth(nth).unwrap();
                        if end_node.connections.len() == 1 {
                            break next_track;
                        }
                        if next_track.0 != &train.current_track {
                            break next_track;
                        }
                    };

                    train.current_track = *next_track.0;
                    train.direction = !*next_track.1;

                    train.progress = match train.direction {
                        Direction::Forward => 0f64,
                        Direction::Backward => 1f64,
                    };
                    flag = true;
                } else {
                    train.progress += move_distance
                        / tracks.get(&train.current_track).unwrap().length
                        * match train.direction {
                            Direction::Forward => 1f64,
                            Direction::Backward => -1f64,
                        };
                    break;
                }
            }
            return flag;
        }

        fn to_packet(&self, id: u32, tracks: &BTreeMap<u32, TrackPiece>) -> ServerPacket {
            ServerPacket::PacketTRAIN(
                id,
                self.current_track,
                self.progress,
                Duration::from_secs_f64(
                    tracks.get(&self.current_track).unwrap().length / self.properties.speed,
                ),
                self.direction,
                match self.direction {
                    Direction::Forward => self.properties.image_forward.clone(),
                    Direction::Backward => self.properties.image_backward.clone(),
                },
            )
        }
    }

    println!("Server Started");

    let mut trains = {
        let train_vec = vec![
            TrainInstance {
                properties: TrainProperties {
                    speed: 500f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 0,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 250f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 0,
                progress: 0.0,
                direction: Direction::Backward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 250f64,
                    image_forward: "train2_right.png".into(),
                    image_backward: "train2_left.png".into(),
                },
                current_track: 0,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 490f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 2,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 480f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 4,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 470f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 6,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 460f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 8,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 450f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 10,
                progress: 0.0,
                direction: Direction::Forward,
            },
            TrainInstance {
                properties: TrainProperties {
                    speed: 400f64,
                    image_forward: "train_right_debug.png".into(),
                    image_backward: "train_left_debug.png".into(),
                },
                current_track: 12,
                progress: 0.0,
                direction: Direction::Forward,
            },
        ];
        let mut trains = BTreeMap::<TrainID, TrainInstance>::new();
        for (id, train) in train_vec.into_iter().enumerate() {
            trains.insert(id as u32, train);
        }

        trains
    };

    let valid_train_id = trains.keys().cloned().collect();
    valid_id_tx.send(valid_train_id).unwrap();

    let mut nodes: BTreeMap<NodeID, Node> = {
        let node_coords = vec![
            Coord(2000f64, 100f64),
            Coord(2800f64, 500f64),
            Coord(2200f64, 550f64),
            Coord(1800f64, 350f64),
            Coord(1300f64, 400f64),
            Coord(1000f64, 400f64),
            Coord(300f64, 400f64),
            Coord(-200f64, 300f64),
            Coord(-1175f64, 550f64),
            Coord(-1500f64, 400f64),
            Coord(-2150f64, 450f64),
            Coord(-2800f64, 100f64),
            Coord(-2100f64, 100f64),
            Coord(-1800f64, 350f64),
            Coord(-1700f64, 100f64),
            Coord(-1200f64, 300f64),
            Coord(-900f64, 100f64),
            Coord(-400f64, 200f64),
            Coord(400f64, 200f64),
            Coord(750f64, 200f64),
            Coord(900f64, 200f64),
            Coord(1300f64, 300f64),
            Coord(1700f64, 200f64),
        ];
        let mut tracks = BTreeMap::new();
        for (id, coord) in node_coords.into_iter().enumerate() {
            tracks.insert(
                id as u32,
                Node {
                    id: id as u32,
                    coord,
                    connections: BTreeMap::new(),
                },
            );
        }
        tracks
    };

    let tracks = {
        let tracks_diff = [
            BezierDiff::ToBezier4(Coord(2200f64, 400f64), Coord(2900f64, 200f64)),
            //2
            BezierDiff::ToBezier4(Coord(2400f64, 300f64), Coord(2400f64, 550f64)),
            // 3
            BezierDiff::ToBezier4(Coord(2000f64, 550f64), Coord(2100f64, 450f64)),
            // 4
            BezierDiff::ToBezier2,
            // 5
            BezierDiff::ToBezier3(Coord(1200f64, 550f64)),
            // 6
            BezierDiff::ToBezier3(Coord(650f64, 300f64)),
            // 7
            BezierDiff::ToBezier4(Coord(175f64, 550f64), Coord(-200f64, 550f64)),
            // 8
            BezierDiff::ToBezier3(Coord(-445f64, 500f64)),
            // 9
            BezierDiff::ToBezier2,
            // 10
            BezierDiff::ToBezier2,
            // 11
            BezierDiff::ToBezier3(Coord(-2600f64, 550f64)),
            // 12
            BezierDiff::ToBezier2,
            // 13
            BezierDiff::ToBezier4(Coord(-1900f64, 150f64), Coord(-2000f64, 300f64)),
            // 14
            BezierDiff::ToBezier4(Coord(-1700f64, 350f64), Coord(-1700f64, 300f64)),
            // 15
            BezierDiff::ToBezier4(Coord(-1500f64, 100f64), Coord(-1600f64, 300f64)),
            // 16
            BezierDiff::ToBezier4(Coord(-1100f64, 300f64), Coord(-950f64, 200f64)),
            // 17
            BezierDiff::ToBezier4(Coord(-800f64, 100f64), Coord(-700f64, 150f64)),
            // 18
            BezierDiff::ToBezier4(Coord(0f64, 200f64), Coord(0f64, 50f64)),
            // 19
            BezierDiff::ToBezier2,
            // 20
            BezierDiff::ToBezier3(Coord(800f64, 300f64)),
            // 21
            BezierDiff::ToBezier4(Coord(1100f64, 100f64), Coord(1100f64, 300f64)),
            // 22
            BezierDiff::ToBezier2,
            // 23
            BezierDiff::ToBezier3(Coord(1900f64, 250f64)),
        ];
        let mut tracks = BTreeMap::new();
        for (id, diff) in tracks_diff.into_iter().enumerate() {
            tracks.insert(
                id as u32,
                TrackPiece::new(
                    id as u32,
                    id as u32,
                    if id != 22 { id + 1 } else { 0 } as u32,
                    &mut nodes,
                    diff,
                    "#6FC".into(),
                    20f64,
                ),
            );
        }

        tracks.insert(
            23,
            TrackPiece::new(
                23,
                0,
                3,
                &mut nodes,
                BezierDiff::ToBezier2,
                "#6CC".into(),
                20f64,
            ),
        );

        tracks.insert(
            24,
            TrackPiece::new(
                24,
                7,
                17,
                &mut nodes,
                BezierDiff::ToBezier2,
                "#6CC".into(),
                20f64,
            ),
        );

        nodes.insert(
            23,
            Node {
                id: 23,
                coord: Coord(2800f64, 100f64),
                connections: BTreeMap::new(),
            },
        );

        tracks.insert(
            25,
            TrackPiece::new(
                25,
                0,
                23,
                &mut nodes,
                BezierDiff::ToBezier2,
                "#C66".into(),
                20f64,
            ),
        );

        tracks.insert(
            26,
            TrackPiece::new(
                26,
                19,
                20,
                &mut nodes,
                BezierDiff::ToBezier3(Coord(800f64, 100f64)),
                "#6CC".into(),
                20f64,
            ),
        );

        tracks
    };

    let mut viewer_channels: BTreeMap<u32, mpsc::Sender<ServerPacket>> = BTreeMap::new();
    let mut next_viewer_serial = 0u32;
    let (click_tx, mut click_rx) = mpsc::channel::<(TrainID, ClickModifier)>(32);
    let (ctrl_tx, mut ctrl_rx) = mpsc::channel::<CtrlPacket>(64);

    loop {
        let wait_start = tokio::time::Instant::now();

        // calculate when will the next train reach the end of it's current track
        let wait_time = if !trains.is_empty() {
            trains
                .values()
                .map(|train| train.estimated_time_left(&tracks))
                .min()
                .unwrap()
        } else {
            Duration::MAX
        };

        let wait = tokio::time::sleep(wait_time);
        tokio::select! {
            biased;

            _ = wait => {
                let wait_end = tokio::time::Instant::now();
                for (i, train) in trains.iter_mut() {
                    if train.move_with_time(wait_end - wait_start, &nodes, &tracks) {
                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(*i, &tracks)).await;
                        }
                    }
                }
            }

            clicked = click_rx.recv() => {
                let (clicked_id, modifier) = clicked.unwrap();
                println!("Train#{} is clicked, \n {:?}", clicked_id, modifier);

                let wait_end = tokio::time::Instant::now();

                if modifier.ctrl {
                    use rand::prelude::SliceRandom;

                    let _ = trains.remove(&clicked_id);
                    let removal_type = *{[RemovalType::Vibrate, RemovalType::TakeOff,RemovalType::Derail].choose(&mut thread_rng()).unwrap()};
                    let packet = ServerPacket::PacketREMOVE(clicked_id, removal_type);
                    for channel in viewer_channels.values() {
                        channel.send(packet.clone()).await;
                    }
                }

                for (&i, train) in trains.iter_mut() {
                    if i == clicked_id && !modifier.ctrl && !modifier.shift{
                        if train.move_with_time(wait_end - wait_start + Duration::from_secs(3), &nodes, &tracks) {
                            for (_, channel) in viewer_channels.iter() {
                                channel.send(train.to_packet(i, &tracks)).await;
                            }
                        }
                    } else
                    if train.move_with_time(wait_end - wait_start, &nodes, &tracks) {
                        for channel in viewer_channels.values() {
                            channel.send(train.to_packet(i as u32, &tracks)).await;
                        }
                    }
                }

                if modifier.shift && !modifier.ctrl {
                    let clicked = trains.get_mut(&clicked_id).unwrap();
                    clicked.direction = !clicked.direction;
                    for channel in viewer_channels.values() {
                        channel.send(clicked.to_packet(clicked_id, &tracks)).await;
                    }
                }
            }

            ctrl_packet = ctrl_rx.recv() => {
                let ctrl_packet = ctrl_packet.unwrap();
                // TODO: actually handle changes sent by the client
            }

            request_result = view_request_rx.recv() => {
                // received new view request
                let response_tx = request_result.unwrap();
                let (notify_tx, notify_rx) = mpsc::channel(4);

                response_tx.send((notify_rx, click_tx.clone())).unwrap();
                for (_, node) in nodes.iter() {
                    notify_tx.send(node.to_packet()).await.unwrap();
                }
                notify_tx.send(ServerPacket::PacketTRACK(tracks.iter().map(
                    |a| (*a.0, a.1.path, a.1.color.clone(), a.1.thickness)
                ).collect())).await.unwrap();

                let wait_end = tokio::time::Instant::now();
                for (&i, train) in trains.iter_mut() {
                    if train.move_with_time(wait_end - wait_start, &nodes, &tracks) {
                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(i, &tracks)).await;
                        }
                    }
                    notify_tx.send(train.to_packet(i as u32, &tracks)).await;
                }
                viewer_channels.insert(next_viewer_serial, notify_tx);
                next_viewer_serial += 1;
            }

            request_result = ctrl_request_rx.recv() => {
                // received new view request
                let response_tx = request_result.unwrap();
                response_tx.send(ctrl_tx.clone()).unwrap();

                let wait_end = tokio::time::Instant::now();
                for (&i, train) in trains.iter_mut() {
                    if train.move_with_time(wait_end - wait_start, &nodes, &tracks) {
                        for (_, channel) in viewer_channels.iter() {
                            channel.send(train.to_packet(i, &tracks)).await;
                        }
                    }
                }
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
    loop {
        let (view_request_tx, view_request_rx) = mpsc::channel(32);

        let (ctrl_request_tx, ctrl_request_rx) = mpsc::channel(32);

        let (valid_id_tx, valid_id_rx) = watch::channel(BTreeSet::new());

        let (derail_tx, derail_rx) = mpsc::channel(1);

        let tm_handle = tokio::spawn(async move {
            train_master(view_request_rx, ctrl_request_rx, valid_id_tx, derail_rx).await
        });

        // build our application with a single route

        let shared_state = AppState {
            view_request_tx,
            valid_id: valid_id_rx,
            ctrl_request_tx,
            derail_tx,
        };

        let assets_dir = std::path::PathBuf::from("../frontend/");

        let app: Router = Router::new()
            .fallback_service(axum::routing::get_service(
                tower_http::services::ServeDir::new(assets_dir)
                    .append_index_html_on_directories(true),
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
            .route("/ws", get(ws_get_handler))
            .route("/ws-ctrl", get(ctrl_get_handler))
            .route("/force-derail", get(derail_handler))
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
