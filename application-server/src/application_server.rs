use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use lorawan::utils::PrettyHexSlice;
use serde::{Serialize, Deserialize};
use tokio::{sync::mpsc, net::{TcpListener, TcpStream}, io::AsyncReadExt};
use tokio::sync::mpsc::Sender as MpscSender;

use crate::utils::{ACTaskCommand, CommandWrapper, self, error::ASError, ASTaskResponse};

#[derive(Serialize, Deserialize)]
struct ASMessage {
    payload: Vec<u8>,
    dev_addr: [u8; 4],
    tmst: u32
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct ApplicationServerConfig {
    pub tcp_receive_port: u16
}

pub struct ApplicationServer {
    config: &'static ApplicationServerConfig
}

impl ApplicationServer {
    pub async fn init(config: &'static ApplicationServerConfig) -> Self {
        Self {
            config
        } 
    }

    async fn handle_data_up(data_up: Vec<u8>) {
        println!("{:?}", String::from_utf8_lossy(&data_up));
        
        let uplink_message: ASMessage = serde_json::from_slice(&data_up).unwrap();
        println!("{:?}", String::from_utf8_lossy(&uplink_message.payload));
    }
    
    async fn handle_data_down(data_down: Vec<u8>) {
        println!("{:?}", String::from_utf8_lossy(&data_down));
        //TODO send in downlink --> E COME?
        //lista dei gateway con dns tipo <id_gateway>.dlwan.phd e salvataggio ID dei gateway che lo hanno ricevuto. 
        //Dove lo prendo? dalla blockchain? Mi serve anche questo come nodo?
    }

    async fn handle_commands_task(mut mpsc_rx: mpsc::Receiver<CommandWrapper>) {
        while let Some(command) = mpsc_rx.recv().await {
            tokio::spawn(async move {
                if (match command.0 {
                    ACTaskCommand::DataUp { data_up } => {
                        Self::handle_data_up(data_up).await;
                        command.1.send(ASTaskResponse::DataUp).unwrap();
                        Ok(())
                    },
                    ACTaskCommand::DataDown { data_down } => {
                        Self::handle_data_down(data_down).await;
                        Err(1)
                    },
                }).is_err() { eprintln!("Error sending response back to task handler") }
            }); //end task
        }
    }

    async fn handle_new_connection(mut cl_sock: TcpStream, mpsc_tx: MpscSender<CommandWrapper>) {
        let mut buf = [0_u8; 1024];
        while let Ok(bytes_read) = cl_sock.read(&mut buf).await {
            println!("read {} bytes: {}", bytes_read, PrettyHexSlice(&buf[..bytes_read]));
            if bytes_read == 0 {break}
            let cmd = ACTaskCommand::DataUp { data_up: buf[0..bytes_read].to_vec() };
            if let Err(e) = utils::send_task(cmd, &mpsc_tx).await {
                eprintln!("{e:?}");
            }
        }
        println!("task ended");
    }

    pub async fn routine(&self) -> Result<(), ASError> {
        let socket_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.config.tcp_receive_port);
        let receiving_socket = TcpListener::bind(socket_addr).await.unwrap();
        
        let (mpsc_tx, mpsc_rx) = mpsc::channel::<CommandWrapper>(128);

        tokio::spawn(Self::handle_commands_task(mpsc_rx));

        println!("Waiting for connections...");
        while let Ok((cl_sock, _addr)) = receiving_socket.accept().await {
            let mpsc_tx_clone = mpsc_tx.clone();
            tokio::spawn(Self::handle_new_connection(cl_sock, mpsc_tx_clone));
        };
        Ok(())
    }

}