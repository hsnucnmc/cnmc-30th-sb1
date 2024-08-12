pub mod packet;
pub mod handler;

use std::collections::BTreeSet;

use tokio::sync::{mpsc, oneshot, watch};
use packet::*;

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
}
