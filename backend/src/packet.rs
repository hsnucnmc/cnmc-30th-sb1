pub type ImageSrc = String;
pub type TrainID = u32;
pub type TrackID = u32;
pub type Color = String;
pub type Thickness = f64;
pub type StartT = f64;
pub type Duration = tokio::time::Duration; // ms

#[derive(Debug, Clone, Copy)]
pub struct Coord(pub f64, pub f64); // ms

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{}", self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Bezier {
    Bezier2(Coord, Coord),
    Bezier3(Coord, Coord, Coord),
    Bezier4(Coord, Coord, Coord, Coord),
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

#[derive(Debug, Clone)]
pub enum ServerPacket {
    PacketTRAIN(TrainID, TrackID, StartT, Duration, ImageSrc),
    PacketTRACK(Vec<(TrackID, Bezier, Color, Thickness)>),
}

impl std::fmt::Display for ServerPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PacketTRAIN(train_id, track_id, start_t, duration, image_src) => {
                write!(f, "train\n{} {} {} {}\n{}", train_id, track_id, start_t, duration.as_secs_f64() * 1000f64, image_src)
            }

            Self::PacketTRACK(tracks) => {
                write!(f, "track\n{}", tracks.len())?;
                for track in tracks {
                    write!(f, "\n{} {} {} {}", track.0, track.1, track.2, track.3)?;
                }
                Ok(())
            }
        }
    }
}

impl From<ServerPacket> for axum::extract::ws::Message {
    fn from(packet: ServerPacket) -> Self {
        axum::extract::ws::Message::Text(packet.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClientPacket {
    PacketCLICK(TrainID),
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
                let id = match split[1].parse() {
                    Ok(id) => id,
                    Err(_) => return Err("Packet contains a bad train id number"),
                };

                Ok(ClientPacket::PacketCLICK(id))
            }
            _ => Err("Packet contained a unexpected type identifier"),
        }
    }
}
