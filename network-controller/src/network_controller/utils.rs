//use std::net::{SocketAddrV4, Ipv4Addr, SocketAddr};

use lorawan::utils::eui::EUI64;
use tokio::sync::oneshot; // net::TcpSocket, io::AsyncWriteExt};
use crate::utils::error::NCError;
use crate::network_controller::MpscSender;
use tokio::sync::oneshot::Sender as OneshotSender;

#[derive(Debug)]
pub enum NCTaskCommand {
    JoinRequest { join_request: Vec<u8> },
    UnConfirmedDataUp { data_up: Vec<u8> },
    ConfirmedDataUp { data_up: Vec<u8> },
}

#[derive(Debug)]
pub enum NCTaskResponse {
    JoinRequest { result: Result<(Vec<u8>, EUI64), NCError> },
    UnConfirmedDataUp { result: Result<(), NCError> },
    ConfirmedDataUp { result: Result<(Vec<u8>, EUI64), NCError> }
}
pub struct CommandWrapper(pub NCTaskCommand, pub OneshotSender<NCTaskResponse>);


pub async fn send_task(cmd: NCTaskCommand, mpsc_tx: &MpscSender<CommandWrapper>) -> Result<NCTaskResponse, NCError> {
    let (os_tx, os_rx) = oneshot::channel::<NCTaskResponse>();
    mpsc_tx.send(CommandWrapper(cmd, os_tx)).await
        .map_err(|e| NCError::CommandTransmissionFailed(e.to_string()))?;
    os_rx.await.map_err(|e| e.into())
}

/*pub async fn uplink_to_application_server(packet: &[u8]) -> Result<(), NCError> {
    let socket = TcpSocket::new_v4().unwrap();
    let mut stream = socket.connect(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9091))).await.unwrap();
    if let Err(e) = stream.write_all(packet).await {
        return Err(NCError::IOError(e));
    }
    Ok(())
}*/