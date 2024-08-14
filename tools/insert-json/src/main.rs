use packet::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string};

#[derive(Serialize, Deserialize)]
struct Node {
    id: NodeID,
    coord: Coord,
    connections: BTreeMap<TrackID, Direction>, // 順向還是反向進入接點；
    conn_type: NodeType,
}

impl Node {
    fn to_packet(&self) -> ServerPacket {
        ServerPacket::PacketNODE(self.id, self.coord)
    }

    fn next_track(&self, current_track: TrackID) -> (TrackID, Direction) {
        (|a: (&u32, &Direction)| (*a.0, !*a.1))(match self.conn_type {
            NodeType::Random => loop {
                let nth = thread_rng().gen_range(0..self.connections.len());
                let next_track = self.connections.iter().nth(nth).unwrap();
                if self.connections.len() == 1 {
                    break next_track;
                }
                if next_track.0 != &current_track {
                    break next_track;
                }
            },
            NodeType::RoundRobin => self
                .connections
                .range(current_track + 1..=TrackID::MAX)
                .next()
                .unwrap_or(self.connections.range(0..=TrackID::MAX).next().unwrap()),
            NodeType::Reverse => (
                &current_track,
                self.connections.get(&current_track).unwrap(),
            ),
        })
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
        let packet = CtrlPacket::NewNode(node.coord, node.conn_type);
        sender.send(ewebsock::WsMessage::Text(packet.to_string()));
    }

    for (id, track) in tracks {
        let packet_new = CtrlPacket::NewTrack(track.start, track.end, track.color);
        let packet_diff = CtrlPacket::TrackAdjust(id, track.path.get_diff());

        sender.send(ewebsock::WsMessage::Text(packet_new.to_string()));
        sender.send(ewebsock::WsMessage::Text(packet_diff.to_string()));
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
    sender.close();
    receiver.try_recv().unwrap();
}
