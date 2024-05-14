use std::{net::SocketAddr, time::Duration};
use consensus::{consensus_server::{ConsensusConfig, ConsensusServer}, get_addr, ConsensusCerts, ConsensusMessage};
use tokio::sync::mpsc::{Receiver, Sender};
use openssl::sha::Sha256;

const NUM_NC: u32 = 16;

struct MockServer {
    msg_receiver: Receiver<ConsensusMessage>,
    consensus_sender: Sender<ConsensusMessage>,
}

impl MockServer {
    pub fn new(msg_receiver: Receiver<ConsensusMessage>, id: String, certs: ConsensusCerts, addr: SocketAddr) -> Self {
        let config = ConsensusConfig {
            addr,
            certs,
        };

        let consensus_sender = ConsensusServer::run_instance(id.clone(), config).unwrap();
        MockServer {
            msg_receiver,
            consensus_sender,
        }
    }

    pub async fn run(&mut self) {
        while let Some(msg) = self.msg_receiver.recv().await {
            //self.b.wait().await;
            self.consensus_sender.send(msg).await.unwrap();
        }
    }
}

async fn server(msg_receiver: Receiver<ConsensusMessage>, id: String) {
    let addr = get_addr(&id);

    let certs = ConsensusCerts {
        cert_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{id}/tls/server.crt"),
        key_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{id}/tls/server.key"),
        ca_cert_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{id}/tls/ca.crt"),
    };

    MockServer::new(msg_receiver, id, certs, addr).run().await;
}

fn spawn_server(i: u32) -> Sender<ConsensusMessage> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(server(rx, format!("peer{}.org1.dlwan.phd", i)));
    tx
}

async fn async_main() {
    let mut v = Vec::new();
    let mut nc_list = Vec::new();
    let mut rssi = Vec::new();
    
    for i in 0..NUM_NC {
        v.push(spawn_server(i));
        nc_list.push(format!("peer{}.org1.dlwan.phd", i));
        rssi.push(((i as f32 * -10.0) * 1000.0) as i32)
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut dev_addr_base = [66_u8,66,66,66,66,66,66,66];
    let mut packet_base = [0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    let mut receiverz = Vec::new();

    loop {
        let dev_addr_new = String::from_utf8_lossy(&dev_addr_base).to_string();

        let packet = {
            let mut hasher = Sha256::new();
            hasher.update(&packet_base);
            hasher.finish()
        };

        for (i, tx) in v.iter().enumerate() {
            let (tx_end, rx_end) = tokio::sync::oneshot::channel();
            tx.send(ConsensusMessage {  
                nc_list: nc_list.clone(),
                dev_addr: dev_addr_new.clone(),
                packet: packet.to_vec(),
                rssi: rssi[i],
                response: tx_end,
            }).await.unwrap();
            receiverz.push(rx_end);
        }

        for (i, rx) in receiverz.drain(..).enumerate() {
            println!("[{i}]: {}",rx.await.unwrap());
        }

        for v in dev_addr_base.iter_mut().chain(packet_base.iter_mut()) {
            *v += 1;
        }

        
        
        tokio::time::sleep(Duration::from_secs(20)).await;
        println!("\n\n######## ROUND ######");
    }
}


fn main() {
    tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(async_main());
}
