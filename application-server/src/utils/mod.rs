
pub mod error;

use tokio::sync::{mpsc::Sender as MpscSender, oneshot::{Sender as OneshotSender, self}};
use self::error::ASError;

#[derive(Debug)]
pub enum ACTaskCommand {
    DataUp { data_up: Vec<u8> },
    DataDown { data_down: Vec<u8> },
}

#[derive(Debug)]
pub enum ASTaskResponse {
    DataUp,
    DataDown { data_down: Vec<u8> },
}
pub struct CommandWrapper(pub ACTaskCommand, pub OneshotSender<ASTaskResponse>);

pub async fn send_task(cmd: ACTaskCommand, mpsc_tx: &MpscSender<CommandWrapper>) -> Result<ASTaskResponse, ASError> {
    let (os_tx, os_rx) = oneshot::channel::<ASTaskResponse>();
    mpsc_tx.send(CommandWrapper(cmd, os_tx)).await
        .map_err(|e| ASError::CommandTransmissionFailed(e.to_string()))?;
    os_rx.await.map_err(|e| e.into())
}


