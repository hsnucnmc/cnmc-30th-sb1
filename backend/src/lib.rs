pub mod handler;
pub mod train;
pub mod routing;
use std::collections::BTreeSet;

use packet::*;
use routing::RoutingInfo;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Clone)]
pub struct AppState {
    pub view_request_tx: mpsc::Sender<
        oneshot::Sender<(
            mpsc::Receiver<ServerPacket>,
            mpsc::Sender<(TrainID, ClickModifier)>,
        )>,
    >,
    pub ctrl_request_tx: mpsc::Sender<oneshot::Sender<mpsc::Sender<CtrlPacket>>>,
    pub valid_id: watch::Receiver<BTreeSet<TrainID>>,
    pub derail_tx: mpsc::Sender<()>,
    pub list_nodes_request: mpsc::Sender<oneshot::Sender<Vec<(NodeID, Coord)>>>,
    pub node_type_request: mpsc::Sender<(NodeID, oneshot::Sender<NodeType>)>,
    pub node_get_routing_request: mpsc::Sender<(NodeID, oneshot::Sender<Option<RoutingInfo>>)>,
    pub node_set_routing_request: mpsc::Sender<(NodeID, RoutingInfo)>,
    pub next_track: std::sync::Arc<tokio::sync::Mutex<Option<String>>>,
}
