use rand::prelude::Distribution;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{read_to_string, File};
use std::io::Write;
use tokio::sync::{mpsc, oneshot, watch};

use packet::*;

use crate::routing::{AfterEffects, BuiltRouter, CompoundRoutingType, RoutingInfo, RoutingType};

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MultiDirection {
    Both,
    Forward,
    Backward,
}

impl MultiDirection {
    fn has_direction(&self, direction: Direction) -> bool {
        match self {
            MultiDirection::Both => true,
            MultiDirection::Forward => direction == Direction::Forward,
            MultiDirection::Backward => direction == Direction::Backward,
        }
    }

    fn add_direction(&mut self, direction: Direction) {
        *self = match self {
            MultiDirection::Both => MultiDirection::Both,
            MultiDirection::Forward => {
                if direction == Direction::Backward {
                    MultiDirection::Both
                } else {
                    MultiDirection::Forward
                }
            }
            MultiDirection::Backward => {
                if direction == Direction::Forward {
                    MultiDirection::Both
                } else {
                    MultiDirection::Backward
                }
            }
        }
    }
}

impl Distribution<Direction> for MultiDirection {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        if rng.gen_bool(0.5f64) {
            if self.has_direction(Direction::Forward) {
                Direction::Forward
            } else {
                Direction::Backward
            }
        } else {
            if self.has_direction(Direction::Backward) {
                Direction::Backward
            } else {
                Direction::Forward
            }
        }
    }
}

impl From<Direction> for MultiDirection {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Forward => MultiDirection::Forward,
            Direction::Backward => MultiDirection::Backward,
        }
    }
}

#[derive(PartialEq, Clone, Serialize)]
struct Node {
    id: NodeID,
    coord: Coord,
    connections: BTreeMap<TrackID, MultiDirection>, // 順向還是反向進入接點；
    conn_type: NodeType,
    routing_info: Option<RoutingInfo>,
    #[serde(skip)]
    router: Option<BuiltRouter>,
}

impl Node {
    fn strip(self) -> StrippedNode {
        StrippedNode {
            id: self.id,
            coord: self.coord,
            connections: self.connections,
            conn_type: self.conn_type,
            routing_info: self.routing_info,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
struct StrippedNode {
    id: NodeID,
    coord: Coord,
    connections: BTreeMap<TrackID, MultiDirection>, // 順向還是反向進入接點；
    conn_type: NodeType,
    routing_info: Option<RoutingInfo>,
}

impl StrippedNode {
    fn build(self) -> Node {
        Node {
            id: self.id,
            coord: self.coord,
            connections: self.connections,
            conn_type: self.conn_type,
            router: match &self.routing_info {
                Some(routing_info) => Some(routing_info.clone().build()),
                None => None,
            },
            routing_info: self.routing_info,
        }
    }

    fn build_clean(&self) -> Node {
        let router = self
            .routing_info
            .as_ref()
            .and_then(|x| Some(x.clone().build()));
        Node {
            id: self.id,
            coord: self.coord,
            connections: BTreeMap::new(),
            conn_type: self.conn_type,
            router,
            routing_info: self.routing_info.clone(),
        }
    }
}

impl Node {
    fn connect(&mut self, track_id: TrackID, direction: Direction) {
        match self.connections.get_mut(&track_id) {
            None => {
                self.connections.insert(track_id, direction.into());
            }
            Some(multi_direction) => multi_direction.add_direction(direction),
        }
    }

    fn to_packet(&self) -> ServerPacket {
        ServerPacket::PacketNODE(self.id, self.coord)
    }

    fn next_track(&mut self, current_track: TrackID, current_direction: Direction) -> RoutingType {
        match self.conn_type {
            NodeType::Random => loop {
                let nth = thread_rng().gen_range(0..self.connections.len());
                let next_track = self.connections.iter().nth(nth).unwrap();
                if self.connections.len() == 1 {
                    break RoutingType::BounceBack;
                }
                if next_track.0 != &current_track {
                    break RoutingType::Track((
                        *next_track.0,
                        !next_track.1.sample(&mut thread_rng()),
                    ));
                }
            },
            NodeType::RoundRobin => match self.connections.get(&current_track).unwrap() {
                MultiDirection::Both => {
                    if current_direction == Direction::Forward {}
                    todo!()
                }
                MultiDirection::Forward => todo!(),
                MultiDirection::Backward => todo!(),
            },
            NodeType::Reverse => RoutingType::BounceBack,
            // TODO: implement the new node types
            NodeType::Derail => RoutingType::Derail,
            NodeType::Configurable => self
                .router
                .as_mut()
                .unwrap()
                .route(&mut thread_rng(), (current_track, current_direction)),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
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
            .connect(id, Direction::Backward);
        nodes
            .get_mut(&end_id)
            .unwrap()
            .connect(id, Direction::Forward);

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

#[derive(Serialize, Deserialize, Clone)]
struct TrainProperties {
    speed: f64, // px/s
    image_forward: String,
    image_backward: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TrainInstance {
    properties: TrainProperties,
    current_track: u32,
    progress: f64,        // 0 ~ 1
    direction: Direction, // backward direction: progress goes from 1 to 0
}

enum MoveResult {
    Nothing(TrainInstance),
    PassesNode(TrainInstance),
    Derailed,
}

impl MoveResult {
    fn make_packet(
        &self,
        id: TrainID,
        tracks: &BTreeMap<TrackID, TrackPiece>,
    ) -> Option<ServerPacket> {
        match self {
            MoveResult::Nothing(_) => None,
            MoveResult::PassesNode(train) => Some(train.to_packet(id, tracks)),
            MoveResult::Derailed => Some(ServerPacket::PacketREMOVE(id, RemovalType::Derail)),
        }
    }
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
        nodes: &mut BTreeMap<NodeID, Node>,
        tracks: &BTreeMap<TrackID, TrackPiece>,
    ) -> MoveResult {
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
                            .get_mut(&tracks.get(&train.current_track).unwrap().end)
                            .unwrap();
                        assert!(end_node
                            .connections
                            .get(&train.current_track)
                            .unwrap()
                            .has_direction(Direction::Forward));
                        end_node
                    }
                    Direction::Backward => {
                        let end_node = nodes
                            .get_mut(&tracks.get(&train.current_track).unwrap().start)
                            .unwrap();
                        assert!(end_node
                            .connections
                            .get(&train.current_track)
                            .unwrap()
                            .has_direction(Direction::Backward));
                        end_node
                    }
                };

                match end_node.next_track(train.current_track, train.direction) {
                    RoutingType::Derail => return MoveResult::Derailed,
                    RoutingType::BounceBack => {
                        train.direction = !train.direction;
                    }
                    RoutingType::Track((track, direction)) => {
                        train.current_track = track;
                        train.direction = direction;
                    }
                }

                train.progress = match train.direction {
                    Direction::Forward => 0f64,
                    Direction::Backward => 1f64,
                };

                flag = true;
            } else {
                train.progress += move_distance / tracks.get(&train.current_track).unwrap().length
                    * match train.direction {
                        Direction::Forward => 1f64,
                        Direction::Backward => -1f64,
                    };
                break;
            }
        }

        if flag {
            MoveResult::PassesNode(train.clone())
        } else {
            MoveResult::Nothing(train.clone())
        }
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

fn empty_stuff() -> (
    BTreeMap<NodeID, Node>,
    BTreeMap<TrackID, TrackPiece>,
    BTreeMap<TrainID, TrainInstance>,
) {
    (BTreeMap::new(), BTreeMap::new(), BTreeMap::new())
}

fn test_stuff() -> (
    BTreeMap<NodeID, Node>,
    BTreeMap<TrackID, TrackPiece>,
    BTreeMap<TrainID, TrainInstance>,
) {
    let trains = {
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
                    conn_type: NodeType::Random,
                    routing_info: None,
                    router: None,
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
                conn_type: NodeType::Random,
                routing_info: None,
                router: None,
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

    (nodes, tracks, trains)
}

fn config_test_stuff() -> (
    BTreeMap<NodeID, Node>,
    BTreeMap<TrackID, TrackPiece>,
    BTreeMap<TrainID, TrainInstance>,
) {
    let (mut nodes, mut tracks, mut trains) = empty_stuff();
    nodes.insert(0, {
        let routing_info = RoutingInfo {
            configured: true,
            default_state: 0,
            states: vec![(
                0,
                crate::routing::RoutingState {
                    after_click: crate::routing::AfterEffects::Nothing,
                    forward_routings: vec![
                        (
                            3,
                            (
                                CompoundRoutingType::Simple(RoutingType::Track((
                                    0,
                                    Direction::Forward,
                                ))),
                                AfterEffects::Nothing,
                            ),
                        ),
                        (
                            4,
                            (
                                CompoundRoutingType::Simple(RoutingType::Track((
                                    0,
                                    Direction::Forward,
                                ))),
                                AfterEffects::Nothing,
                            ),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    backward_routings: vec![].into_iter().collect(),
                },
            )]
            .into_iter()
            .collect(),
        };
        Node {
            id: 0,
            coord: Coord(0.0, 0.0),
            connections: BTreeMap::new(),
            conn_type: NodeType::Configurable,
            routing_info: Some(routing_info.clone()),
            router: Some(routing_info.build()),
        }
    });

    (nodes, tracks, trains)
}

fn remove_stuff(existing: &mut BTreeSet<String>, stuff_name: &str) {
    existing.remove(stuff_name);
    match File::create("tracks/existing.json") {
        Ok(mut file) => match file.write_all(serde_json::to_string(existing).unwrap().as_bytes()) {
            Ok(_) => {}
            Err(_) => {
                println!("Failed writing to tracks/existing.json");
            }
        },
        Err(_) => {
            println!("Failed writing to tracks/existing.json");
        }
    }
    match std::fs::remove_file(format!("tracks/nodes_{}.json", stuff_name)) {
        Ok(_) => {}
        Err(_) => {
            println!("Failed removing tracks/nodes_{}.json", stuff_name);
        }
    }
    match std::fs::remove_file(format!("tracks/track_{}.json", stuff_name)) {
        Ok(_) => {}
        Err(_) => {
            println!("Failed removing tracks/track_{}.json", stuff_name);
        }
    }
}

fn read_stuff_from_name(
    track_name: &str,
) -> Result<
    (
        BTreeMap<NodeID, Node>,
        BTreeMap<TrackID, TrackPiece>,
        BTreeMap<TrainID, TrainInstance>,
    ),
    &'static str,
> {
    if track_name == "" {
        return Ok(empty_stuff());
    }

    if !track_name
        .chars()
        .all(|chr: char| chr.is_alphanumeric() || chr == '_' || chr == '-')
    {
        return Err("Track name contains invalid character");
    }
    let mut existing: BTreeSet<String> = match serde_json::from_str(
        &std::fs::read_to_string("tracks/existing.json").unwrap_or("[]".into()),
    ) {
        Ok(set) => set,
        Err(_) => {
            return Err("Failed parsing exsiting.json");
        }
    };
    if !existing.contains(track_name) {
        return Err("Track name isn't found in existing.json");
    }

    let loaded_nodes: BTreeMap<u32, StrippedNode> = match serde_json::from_str(
        match &read_to_string(format!("tracks/nodes_{}.json", track_name)) {
            Ok(file) => file,
            Err(_) => {
                remove_stuff(&mut existing, track_name);
                return Err("Failed reading node file");
            }
        },
    ) {
        Ok(nodes) => nodes,
        Err(_) => {
            remove_stuff(&mut existing, track_name);
            return Err("Failed parsing node file");
        }
    };
    let loaded_tracks: BTreeMap<u32, TrackPiece> = match serde_json::from_str(
        match &read_to_string(format!("tracks/track_{}.json", track_name)) {
            Ok(file) => file,
            Err(_) => {
                remove_stuff(&mut existing, track_name);
                return Err("Failed reading track file");
            }
        },
    ) {
        Ok(tracks) => tracks,
        Err(_) => {
            remove_stuff(&mut existing, track_name);
            return Err("Failed parsing track file");
        }
    };

    let mut nodes = BTreeMap::new();
    for (id, node) in &loaded_nodes {
        nodes.insert(node.id, node.build_clean());
    }

    let mut tracks = BTreeMap::new();
    for (id, track) in &loaded_tracks {
        if !nodes.contains_key(&track.start) {
            return Err("Loaded track contains start node not included in nodes");
        }
        if !nodes.contains_key(&track.end) {
            return Err("Loaded track contains end node not included in nodes");
        }

        tracks.insert(
            *id,
            TrackPiece::new(
                *id,
                track.start,
                track.end,
                &mut nodes,
                track.path.get_diff(),
                track.color.clone(),
                track.thickness,
            ),
        );
    }

    let trains = BTreeMap::new();
    if loaded_nodes
        != nodes
            .iter()
            .map(|(id, node)| (*id, node.clone().strip()))
            .collect()
    {
        println!("Possibly inconsistency existing in loaded nodes but using it anyway");
    }
    if loaded_tracks != tracks {
        println!("Possibly inconsistency existing in loaded nodes but using it anyway");
    }
    Ok((nodes, tracks, trains))
}

fn node_list_request_handler(
    response_tx: oneshot::Sender<Vec<(NodeID, Coord)>>,
    nodes: &BTreeMap<NodeID, Node>,
) {
    let node_list = nodes.iter().map(|(id, node)| (*id, node.coord)).collect();
    let _ = response_tx.send(node_list);
}

fn node_get_type_request_handler(
    response_tx: oneshot::Sender<Option<NodeType>>,
    node_id: NodeID,
    nodes: &BTreeMap<NodeID, Node>,
) {
    let _ = response_tx.send(if let Some(node) = nodes.get(&node_id) {
        Some(node.conn_type)
    } else {
        None
    });
}

fn node_get_routing_request_handler(
    response_tx: oneshot::Sender<Option<RoutingInfo>>,
    node_id: NodeID,
    nodes: &BTreeMap<NodeID, Node>,
) {
    let _ = response_tx.send(if let Some(node) = nodes.get(&node_id) {
        node.routing_info.clone()
    } else {
        None
    });
}

fn node_set_routing_request_handler(
    node_id: NodeID,
    routing_info: RoutingInfo,
    nodes: &mut BTreeMap<NodeID, Node>,
    tracks: &BTreeMap<TrackID, TrackPiece>,
) {
    if let Some(node) = nodes.get_mut(&node_id) {
        if routing_info.check().is_err() {
            return;
        }

        for routing_type in routing_info.outcomes() {
            match routing_type {
                RoutingType::Track((track_id, _)) => {
                    if !tracks.contains_key(&track_id) {
                        return;
                    }
                }
                _ => {}
            }
        }

        node.routing_info = Some(routing_info.clone());
        node.router = Some(routing_info.build());
    }
}

async fn move_single_train_and_notify(
    duration: Duration,
    nodes: &mut BTreeMap<NodeID, Node>,
    tracks: &BTreeMap<TrackID, TrackPiece>,
    id: TrainID,
    train: &mut TrainInstance,
    viewer_channels: &BTreeMap<u32, mpsc::Sender<ServerPacket>>,
) -> bool {
    let result = train.move_with_time(duration, nodes, tracks);
    let packet = result.make_packet(id, &tracks);
    if let Some(packet) = packet {
        for (_, channel) in viewer_channels.iter() {
            channel.send(packet.clone()).await;
        }
    }

    match result {
        MoveResult::Nothing(finished_train) => *train = finished_train,
        MoveResult::PassesNode(finished_train) => *train = finished_train,
        MoveResult::Derailed => {
            return true;
        }
    }

    false
}

async fn move_trains_and_notify(
    duration: Duration,
    nodes: &mut BTreeMap<NodeID, Node>,
    tracks: &BTreeMap<TrackID, TrackPiece>,
    trains: &mut BTreeMap<TrainID, TrainInstance>,
    viewer_channels: &BTreeMap<u32, mpsc::Sender<ServerPacket>>,
) {
    let mut removed = Vec::new();
    for (i, train) in trains.iter_mut() {
        if move_single_train_and_notify(duration, nodes, tracks, *i, train, viewer_channels).await {
            removed.push(*i);
        }
    }

    for i in removed {
        let _ = trains.remove(&i).unwrap();
    }
}
pub async fn train_master(
    mut view_request_rx: mpsc::Receiver<
        oneshot::Sender<(
            mpsc::Receiver<ServerPacket>,
            mpsc::Sender<(TrainID, ClickModifier)>,
        )>,
    >,
    mut ctrl_request_rx: mpsc::Receiver<oneshot::Sender<mpsc::Sender<CtrlPacket>>>,
    valid_id_tx: watch::Sender<BTreeSet<TrainID>>,
    mut derail_rx: mpsc::Receiver<()>,
    mut list_nodes_request_rx: mpsc::Receiver<oneshot::Sender<Vec<(NodeID, Coord)>>>,
    mut node_type_request_rx: mpsc::Receiver<(NodeID, oneshot::Sender<Option<NodeType>>)>,
    mut node_get_routing_request_rx: mpsc::Receiver<(NodeID, oneshot::Sender<Option<RoutingInfo>>)>,
    mut node_set_routing_request_rx: mpsc::Receiver<(NodeID, RoutingInfo)>,
    using_track: String,
) {
    if using_track != "" {
        println!("Server started using track \"{}\"", using_track);
    } else {
        println!("Server started using default track");
    }

    let (mut nodes, mut tracks, mut trains) =
        read_stuff_from_name(&using_track).unwrap_or_else(|err| {
            println!(
                "Failed reading track:\n\t{}\nUsing empty track instead",
                err
            );
            empty_stuff()
        });

    let mut next_node_serial = nodes.len() as u32;
    let mut next_track_serial = tracks.len() as u32;
    let mut next_train_serial = trains.len() as u32;

    let mut valid_train_id: BTreeSet<_> = trains.keys().cloned().collect();
    valid_id_tx.send(valid_train_id.clone()).unwrap();

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
                let duration = tokio::time::Instant::now() - wait_start;
                move_trains_and_notify(duration, &mut nodes, &tracks, &mut trains, &viewer_channels).await;
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

                let duration = wait_end - wait_start;
                {
                    let mut removed = Vec::new();
                    for (i, train) in trains.iter_mut() {
                        if *i == clicked_id && !modifier.ctrl && !modifier.shift{
                            if move_single_train_and_notify(duration + Duration::from_secs(5), &mut nodes, &tracks, *i, train, &viewer_channels).await {
                                removed.push(*i);
                            }
                        } else {
                            if move_single_train_and_notify(duration, &mut nodes, &tracks, *i, train, &viewer_channels).await {
                                removed.push(*i);
                            }
                        }
                    }

                    for i in removed {
                        let _ = trains.remove(&i).unwrap();
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
                match ctrl_packet {
                    CtrlPacket::NewNode(coord, conn_type) => {
                        let mut new_node = Node {
                            id: next_node_serial,
                            coord,
                            connections: BTreeMap::new(),
                            conn_type,
                            routing_info: None,
                            router: None,
                        };

                        if conn_type == NodeType::Configurable {
                            new_node.routing_info = Some(RoutingInfo::default());
                            new_node.router = Some(RoutingInfo::default().build());
                        }

                        for channel in viewer_channels.values() {
                            channel.send(new_node.to_packet()).await;
                        }

                        nodes.insert(next_node_serial, new_node);

                        next_node_serial += 1;
                    },
                    CtrlPacket::NewTrain(track_id, train_speed) => {
                        let new_train = TrainInstance {
                            properties: TrainProperties {
                                speed: train_speed,
                                image_forward: "train_right_debug.png".into(),
                                image_backward: "train_left_debug.png".into(),
                            },
                            current_track: track_id,
                            progress: 0.0,
                            direction: Direction::Forward,
                        };

                        for channel in viewer_channels.values() {
                            channel.send(new_train.to_packet(next_train_serial, &tracks)).await;
                        }

                        trains.insert(next_train_serial, new_train);
                        valid_train_id.insert(next_train_serial);
                        valid_id_tx.send(valid_train_id.clone()).unwrap();

                        next_train_serial += 1;
                    },
                    CtrlPacket::NewTrack(start, end, color) => {
                        if nodes.contains_key(&start) && nodes.contains_key(&end) {
                            let new_track = TrackPiece::new(
                                next_track_serial,
                                start,
                                end,
                                &mut nodes,
                                BezierDiff::ToBezier2,
                                color,
                                20f64,
                            );

                            tracks.insert(next_track_serial, new_track);

                            let packet = ServerPacket::PacketTRACK(
                                tracks
                                    .iter()
                                    .map(|a| (*a.0, a.1.path, a.1.color.clone(), a.1.thickness))
                                    .collect(),
                            );
                            for channel in viewer_channels.values() {
                                channel.send(packet.clone()).await;
                            }

                            next_track_serial += 1;
                        }
                    },
                    CtrlPacket::NodeMove(node_id, coord) => {
                        if let Some(node) = nodes.get_mut(&node_id) {
                            node.coord = coord;
                            for (id, &direction) in &node.connections {
                                let track = tracks.get_mut(id).unwrap();
                                match direction {
                                    MultiDirection::Forward => {
                                        *track.path.end_mut() = coord;
                                    }
                                    MultiDirection::Backward => {
                                        *track.path.start_mut() = coord;
                                    }
                                    MultiDirection::Both => {
                                        *track.path.start_mut() = coord;
                                        *track.path.end_mut() = coord;
                                    }
                                }
                                track.length = track.path.fast_length();
                            }

                            // TODO: The effects of adjusting tracks while there's train on them is ignored
                            for channel in viewer_channels.values() {
                                channel.send(node.to_packet()).await;
                                channel
                                    .send(ServerPacket::PacketTRACK(
                                        tracks
                                            .iter()
                                            .map(|a| (*a.0, a.1.path, a.1.color.clone(), a.1.thickness))
                                            .collect(),
                                    ))
                                    .await;
                        }
                        }
                    }
                    CtrlPacket::TrackAdjust(track_id, diff) => {
                        if let Some(track) = tracks.get_mut(&track_id) {
                            track.path.apply_diff(diff);
                            track.length = track.path.fast_length();
                        }

                        for channel in viewer_channels.values() {
                            channel
                                .send(ServerPacket::PacketTRACK(
                                    tracks
                                        .iter()
                                        .map(|a| (*a.0, a.1.path, a.1.color.clone(), a.1.thickness))
                                        .collect(),
                                ))
                                .await;
                        }
                    }
                    // TODO: implement the new CtrlPackets
                    _ => {},
                }

                let duration = tokio::time::Instant::now() - wait_start;
                move_trains_and_notify(duration, &mut nodes, &tracks, &mut trains, &viewer_channels).await;
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

                let duration = tokio::time::Instant::now() - wait_start;
                move_trains_and_notify(duration, &mut nodes, &tracks, &mut trains, &viewer_channels).await;

                viewer_channels.insert(next_viewer_serial, notify_tx);
                next_viewer_serial += 1;
            }

            request_result = ctrl_request_rx.recv() => {
                // received new view request
                let response_tx = request_result.unwrap();
                response_tx.send(ctrl_tx.clone()).unwrap();

                let duration = tokio::time::Instant::now() - wait_start;
                move_trains_and_notify(duration, &mut nodes, &tracks, &mut trains, &viewer_channels).await;
            }

            request = list_nodes_request_rx.recv() => {
                node_list_request_handler(request.unwrap(), &nodes);
            }

            request = node_type_request_rx.recv() => {
                let request = request.unwrap();
                node_get_type_request_handler(request.1, request.0, &nodes);
            }

            request = node_get_routing_request_rx.recv() => {
                let request = request.unwrap();
                node_get_routing_request_handler(request.1, request.0, &nodes);
            }

            request = node_set_routing_request_rx.recv() => {
                let request = request.unwrap();
                node_set_routing_request_handler(request.0, request.1, &mut nodes, &tracks);
            }

            _ = derail_rx.recv() => {
                println!("RECEIVED DERAIL REQUEST!!!");
                break;
            }
        }
    }

    let timestamp = format!(
        "{:011}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("WTF We're in the past")
            .as_secs()
    );
    let _ = std::fs::create_dir("tracks");
    File::create(format!("tracks/nodes_{}.json", timestamp))
        .unwrap()
        .write_all(serde_json::to_string(&nodes).unwrap().as_bytes())
        .unwrap();
    File::create(format!("tracks/track_{}.json", timestamp))
        .unwrap()
        .write_all(serde_json::to_string(&tracks).unwrap().as_bytes())
        .unwrap();
    let mut existing: BTreeSet<String> = serde_json::from_str(
        &std::fs::read_to_string("tracks/existing.json").unwrap_or("[]".into()),
    )
    .unwrap();
    existing.insert(timestamp);
    File::create("tracks/existing.json")
        .unwrap()
        .write_all(serde_json::to_string(&existing).unwrap().as_bytes())
        .unwrap();
}
