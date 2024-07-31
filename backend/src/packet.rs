use ordered_float::OrderedFloat;

#[derive(Debug, Clone, Copy)]
pub struct TrainView {
    pub left: OrderedFloat::<f64>,
    pub right: OrderedFloat::<f64>,
}

#[derive(Debug, Clone)]
pub enum ClientPacket {
    PacketPOSITION(TrainView),
}

impl std::str::FromStr for ClientPacket {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<ClientPacket, Self::Err> {
        if input.split("\n").count() != 2 {
            return Err("Packet has unexpected amount of lines");
        }

        let split: Vec<&str> = input.split("\n").collect();

        match split[0] {
            "position" => {
                if input.split("\n").count() != 2 {
                    return Err("Packet has unexpected amount of lines");
                }

                if split[1].split(" ").count() != 2 {
                    return Err("Packet has unexpected amount of whitespaces");
                }

                let info_split: Vec<&str> = split[1].split(" ").collect();

                let left = match info_split[0].parse() {
                    Ok(left) => left,
                    Err(_) => return Err("Packet contains a bad left boundary"),
                };

                let right = match info_split[1].parse() {
                    Ok(right) => right,
                    Err(_) => return Err("Packet contains a bad right boundary"),
                };

                if left >= right {
                    return Err("Packet contains an invalid range")
                }

                Ok(ClientPacket::PacketPOSITION(TrainView { left, right }))
            }
            _ => Err("Packet contained a unexpected type identifier"),
        }
    }
}

pub type MoveTime = f64;
pub type YPos = f64;
pub type ImageSrc = String;

#[derive(Debug, Clone)]
pub enum ServerPacket {
    PacketLEFT(MoveTime, YPos, ImageSrc),
    PacketRIGHT(MoveTime, YPos, ImageSrc),
}

impl std::fmt::Display for ServerPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PacketLEFT(move_time, y_pos, image_src) => {
                write!(f, "left\n{} {} {}", move_time, y_pos, image_src)
            }

            Self::PacketRIGHT(move_time, y_pos, image_src) => {
                write!(f, "right\n{} {} {}", move_time, y_pos, image_src)
            }
        }
    }
}

impl From<ServerPacket> for axum::extract::ws::Message {
    fn from(packet: ServerPacket) -> Self {
        axum::extract::ws::Message::Text(packet.to_string())
    }
}
