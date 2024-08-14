use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string};
use packet::*;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct TrainProperties {
    speed: f64, // px/s
    image_forward: String,
    image_backward: String,
}

#[derive(Serialize, Deserialize)]
struct TrainInstance {
    properties: TrainProperties,
    current_track: u32,
    progress: f64,        // 0 ~ 1
    direction: Direction, // backward direction: progress goes from 1 to 0
}

fn main() {
    let mut timestamp = String::new();
    let mut location = String::new();
    println!("timestamp? (ex. 01723582454)");
    std::io::stdin().read_line(&mut timestamp).unwrap();
    println!("location? (ex. ws://127.0.0.1:8080)");
    std::io::stdin().read_line(&mut location).unwrap();
    if location.trim() == "" {
        location = "ws://127.0.0.1:8080".into();
    }
    let timestamp: i32 = timestamp.trim().parse().unwrap_or(01723582454);

    let nodes: BTreeMap<u32, Node> = serde_json::from_str(
        &read_to_string(format!("tracks/nodes_{:011}.json", timestamp)).unwrap(),
    )
    .unwrap();
    let tracks: BTreeMap<u32, TrackPiece> = serde_json::from_str(
        &read_to_string(format!("tracks/track_{:011}.json", timestamp)).unwrap(),
    )
    .unwrap();

    let (mut sender, receiver) = ewebsock::connect(
        format!("{}/ws-ctrl", location.trim()),
        ewebsock::Options {
            max_incoming_frame_size: usize::MAX,
        },
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));

    for (_, node) in nodes {
        let packet = CtrlPacket::NewNode(node.coord);
        sender.send(ewebsock::WsMessage::Text(packet.to_string()));
    }

    for (id, track) in tracks {
        let packet_new = CtrlPacket::NewTrack(track.start, track.end);
        let packet_diff = CtrlPacket::TrackAdjust(id, track.path.get_diff());

        sender.send(ewebsock::WsMessage::Text(packet_new.to_string()));
        sender.send(ewebsock::WsMessage::Text(packet_diff.to_string()));
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
    sender.close();
    receiver.try_recv().unwrap();
}
