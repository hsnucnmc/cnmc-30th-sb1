use serde::{Deserialize, Serialize};

pub type ImageSrc = String;
pub type TrainID = u32;
pub type TrackID = u32;
pub type NodeID = u32;
pub type Color = String;
pub type Thickness = f64;
pub type StartT = f64;
pub type Duration = std::time::Duration; // ms

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Coord(pub f64, pub f64); // ms

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{}", self.0, self.1)
    }
}

impl Coord {
    fn distance_to(&self, coord: &Coord) -> f64 {
        ((self.0 - coord.0).powi(2) + (self.1 - coord.1).powi(2)).sqrt()
    }
}

impl std::str::FromStr for Coord {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Coord, Self::Err> {
        if input.split(";").count() != 2 {
            return Err("Coord has unexpected amount of semicolons");
        }

        let mut split = input.split(";");
        let x = match split.next().unwrap().parse() {
            Ok(x) => x,
            Err(_) => return Err("Coord contains a bad x coordinate"),
        };
        let y = match split.next().unwrap().parse() {
            Ok(y) => y,
            Err(_) => return Err("Coord contains a bad y coordinate"),
        };

        Ok(Coord(x, y))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Backward,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Direction::Forward => "forward",
                Direction::Backward => "backward",
            }
        )
    }
}

impl std::ops::Not for Direction {
    type Output = Direction;
    fn not(self) -> Direction {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Bezier {
    Bezier2(Coord, Coord),
    Bezier3(Coord, Coord, Coord),
    Bezier4(Coord, Coord, Coord, Coord),
}

impl Bezier {
    #[inline]
    pub fn fast_length(&self) -> f64 {
        match self {
            Bezier::Bezier2(p1, p2) => p1.distance_to(p2),
            Bezier::Bezier3(p1, p2, p3) => {
                (p1.distance_to(p2) + p2.distance_to(p3) + p1.distance_to(p3)) / 2.0
            }
            Bezier::Bezier4(p1, p2, p3, p4) => {
                (p1.distance_to(p2) + p2.distance_to(p3) + p3.distance_to(p4) + p1.distance_to(p4))
                    / 2.0
            }
        }
    }

    #[inline]
    pub fn start(&self) -> &Coord {
        match self {
            Self::Bezier2(start, _) => start,
            Self::Bezier3(start, _, _) => start,
            Self::Bezier4(start, _, _, _) => start,
        }
    }

    #[inline]
    pub fn end(&self) -> &Coord {
        match self {
            Self::Bezier2(_, end) => end,
            Self::Bezier3(_, _, end) => end,
            Self::Bezier4(_, _, _, end) => end,
        }
    }

    #[inline]
    pub fn start_mut(&mut self) -> &mut Coord {
        match self {
            Self::Bezier2(start, _) => start,
            Self::Bezier3(start, _, _) => start,
            Self::Bezier4(start, _, _, _) => start,
        }
    }

    #[inline]
    pub fn end_mut(&mut self) -> &mut Coord {
        match self {
            Self::Bezier2(_, end) => end,
            Self::Bezier3(_, _, end) => end,
            Self::Bezier4(_, _, _, end) => end,
        }
    }

    #[inline]
    pub fn apply_diff(&mut self, diff: BezierDiff) {
        let start = self.start().clone();
        let end = self.end().clone();

        *self = match diff {
            BezierDiff::ToBezier2 => Self::Bezier2(start, end),
            BezierDiff::ToBezier3(p) => Self::Bezier3(start, p, end),
            BezierDiff::ToBezier4(p1, p2) => Self::Bezier4(start, p1, p2, end),
        };
    }

    pub fn new(start: Coord, end: Coord, diff: BezierDiff) -> Bezier {
        match diff {
            BezierDiff::ToBezier2 => Bezier::Bezier2(start, end),
            BezierDiff::ToBezier3(p) => Bezier::Bezier3(start, p, end),
            BezierDiff::ToBezier4(p1, p2) => Bezier::Bezier4(start, p1, p2, end),
        }
    }

    pub fn get_diff(&self) -> BezierDiff {
        match self {
            &Self::Bezier2(_, _) => BezierDiff::ToBezier2,
            &Self::Bezier3(_, p, _) => BezierDiff::ToBezier3(p),
            &Self::Bezier4(_, p1, p2, _) => BezierDiff::ToBezier4(p1, p2),
        }
    }
}

impl std::fmt::Display for Bezier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bezier2(point0, point1) => {
                write!(f, "bezier2;{};{}", point0, point1)
            }
            Self::Bezier3(point0, point1, point2) => {
                write!(f, "bezier3;{};{};{}", point0, point1, point2)
            }
            Self::Bezier4(point0, point1, point2, point3) => {
                write!(f, "bezier4;{};{};{};{}", point0, point1, point2, point3)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RemovalType {
    Explosion,
    Silent,
    Derail,
    Vibrate,
    TakeOff,
}

impl std::fmt::Display for RemovalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RemovalType::Explosion => "explosion",
                RemovalType::Silent => "silent",
                RemovalType::Derail => "derail",
                RemovalType::Vibrate => "vibrate",
                RemovalType::TakeOff => "take_off",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum ServerPacket {
    PacketTRAIN(TrainID, TrackID, StartT, Duration, Direction, ImageSrc),
    PacketTRACK(Vec<(TrackID, Bezier, Color, Thickness)>),
    PacketNODE(NodeID, Coord),
    PacketREMOVE(TrainID, RemovalType),
}

impl std::fmt::Display for ServerPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PacketTRAIN(train_id, track_id, start_t, duration, direction, image_src) => {
                write!(
                    f,
                    "train\n{} {} {} {} {}\n{}",
                    train_id,
                    track_id,
                    start_t,
                    duration.as_secs_f64() * 1000f64,
                    direction,
                    image_src
                )
            }

            Self::PacketTRACK(tracks) => {
                write!(f, "track\n{}", tracks.len())?;
                for track in tracks {
                    write!(f, "\n{} {} {} {}", track.0, track.1, track.2, track.3)?;
                }
                Ok(())
            }

            Self::PacketNODE(node_id, coord) => {
                write!(f, "node\n{} {}", node_id, coord)
            }

            Self::PacketREMOVE(train_id, removal_type) => {
                write!(f, "remove\n{} {}", train_id, removal_type)
            }
        }
    }
}

impl From<ServerPacket> for axum::extract::ws::Message {
    fn from(packet: ServerPacket) -> Self {
        axum::extract::ws::Message::Text(packet.to_string())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ClickModifier {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl std::str::FromStr for ClickModifier {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<ClickModifier, Self::Err> {
        if input.split(",").count() != 3 {
            return Err("ClickModifier has unexpected amount of commas");
        }

        let split: Vec<&str> = input.split(",").collect();

        let ctrl = match split[0] {
            "0" => false,
            "1" => true,
            _ => return Err("ClickModifier contained a unexpected character"),
        };

        let shift = match split[1] {
            "0" => false,
            "1" => true,
            _ => return Err("ClickModifier contained a unexpected character"),
        };

        let alt = match split[2] {
            "0" => false,
            "1" => true,
            _ => return Err("ClickModifier contained a unexpected character"),
        };

        Ok(ClickModifier { ctrl, shift, alt })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClientPacket {
    PacketCLICK(TrainID, ClickModifier),
}

impl std::str::FromStr for ClientPacket {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<ClientPacket, Self::Err> {
        if input.split("\n").count() != 2 {
            return Err("Packet has unexpected amount of lines");
        }

        let split: Vec<&str> = input.split("\n").collect();

        match split[0] {
            "click" => {
                if split[1].split(" ").count() != 2 {
                    return Err("Packet has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let id = match split_2[0].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet contains a bad train id number"),
                };
                let modifier = match split_2[1].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet contains a bad click modifier"),
                };

                Ok(ClientPacket::PacketCLICK(id, modifier))
            }
            _ => Err("Packet contained a unknown type identifier"),
        }
    }
}

pub enum BezierDiff {
    ToBezier2,
    ToBezier3(Coord),
    ToBezier4(Coord, Coord),
}

impl std::str::FromStr for BezierDiff {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<BezierDiff, Self::Err> {
        if input == "" {
            return Ok(BezierDiff::ToBezier2);
        }
        match input.split(",").count() {
            1 => match input.parse() {
                Ok(coord) => {
                    return Ok(BezierDiff::ToBezier3(coord));
                }
                Err(_) => return Err("BezierDiff contains a bad coordinate"),
            },
            2 => {
                let mut split = input.split(",");
                let p1 = match split.next().unwrap().parse() {
                    Ok(p1) => p1,
                    Err(_) => return Err("Coord contains a bad p1"),
                };
                let p2 = match split.next().unwrap().parse() {
                    Ok(p2) => p2,
                    Err(_) => return Err("Coord contains a bad p2"),
                };
                return Ok(BezierDiff::ToBezier4(p1, p2));
            }
            _ => return Err("BezierDiff has unexpected amount of commas"),
        }
    }
}

impl std::fmt::Display for BezierDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BezierDiff::ToBezier2 => write!(f, ""),
            BezierDiff::ToBezier3(p) => write!(f, "{}", p),
            BezierDiff::ToBezier4(p1, p2) => write!(f, "{},{}", p1, p2),
        }
    }
}

type TrainSpeed = f64;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Random,
    RoundRobin,
    Reverse,
}

impl std::str::FromStr for NodeType {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<NodeType, Self::Err> {
        Ok(match input {
            "random" => Self::Random,
            "roundrobin" => Self::RoundRobin,
            "reverse" => Self::Reverse,
            _ => return Err("Packet contain an unknown node_type"),
        })
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Random => "random",
                Self::RoundRobin => "roundrobin",
                Self::Reverse => "reverse",
            }
        )
    }
}

pub enum CtrlPacket {
    NewNode(Coord, NodeType),
    NewTrain(TrackID, TrainSpeed),
    NewTrack(NodeID, NodeID, Color),
    NodeMove(NodeID, Coord),
    TrackAdjust(TrackID, BezierDiff),
}

impl std::str::FromStr for CtrlPacket {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<CtrlPacket, Self::Err> {
        if input.split("\n").count() != 2 {
            return Err("Packet has unexpected amount of lines");
        }

        let split: Vec<&str> = input.split("\n").collect();

        match split[0] {
            // TODO: check for valid node, train and track ids
            "node_new" => {
                if split[1].split(" ").count() != 2 {
                    return Err("Packet NewNode has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let coord = match split_2[0].parse() {
                    Ok(coord) => coord,
                    Err(_) => return Err("Packet NewNode contains a bad coordinate"),
                };
                
                let node_type = match split_2[1].parse() {
                    Ok(node_type) => node_type,
                    Err(_) => return Err("Packet NewNode contains a bad node_type"),
                };
                
                Ok(CtrlPacket::NewNode(coord, node_type))
            }
            "train_new" => {
                if split[1].split(" ").count() != 2 {
                    return Err("Packet NewTrain has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let track_id = match split_2[0].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet NewTrain contains a bad track_id"),
                };
                let train_speed = match split_2[1].parse() {
                    Ok(train_speed) => train_speed,
                    Err(_) => return Err("Packet NewTrain contains a bad train_speed"),
                };

                Ok(CtrlPacket::NewTrain(track_id, train_speed))
            }
            "track_new" => {
                if split[1].split(" ").count() != 3 {
                    return Err("Packet NewTrack has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let id1 = match split_2[0].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet NewTrack contains a bad node 1 id"),
                };
                let id2 = match split_2[1].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet NewTrack contains a bad node 2 id"),
                };
                let color = split_2[2].to_string();

                Ok(CtrlPacket::NewTrack(id1, id2, color))
            }
            "node_move" => {
                if split[1].split(" ").count() != 2 {
                    return Err("Packet NodeMove has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let id = match split_2[0].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet NodeMove contains a bad node id"),
                };
                let coord = match split_2[1].parse() {
                    Ok(coord) => coord,
                    Err(_) => return Err("Packet NodeMove contains a bad coordinate"),
                };

                Ok(CtrlPacket::NodeMove(id, coord))
            }
            "track_adjust" => {
                if split[1].split(" ").count() != 2 {
                    return Err("Packet TrackAdjust has unexpected amount of whitespaces");
                }

                let split_2: Vec<_> = split[1].split(" ").collect();
                let id = match split_2[0].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet TrackAdjust contains a bad track id"),
                };
                let diff = match split_2[1].parse() {
                    Ok(diff) => diff,
                    Err(_) => return Err("Packet TrackAdjust contains a bad track adjustment"),
                };

                Ok(CtrlPacket::TrackAdjust(id, diff))
            }
            _ => Err("Packet contained a unknown type identifier"),
        }
    }
}

impl std::fmt::Display for CtrlPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CtrlPacket::NewNode(coord, node_type) => {
                write!(f, "node_new\n{} {}", coord, node_type)
            }
            CtrlPacket::NewTrain(track_id, train_speed) => {
                write!(f, "train_new\n{} {}", track_id, train_speed)
            }
            CtrlPacket::NewTrack(start, end, color) => {
                write!(f, "track_new\n{} {} {}", start, end, color)
            }
            CtrlPacket::NodeMove(node_id, coord) => {
                write!(f, "node_move\n{} {}", node_id, coord)
            }
            CtrlPacket::TrackAdjust(track_id, diff) => {
                write!(f, "track_adjust\n{} {}", track_id, diff)
            }
        }
    }
}
