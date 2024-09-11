use std::{collections::HashMap, sync::Arc, time::Duration};

use openssl::{nid::Nid, sha::Sha256, x509::X509};
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        Mutex, RwLock,
    },
    time::Instant,
};
use tonic::{
    transport::{Certificate, CertificateDer, Identity, ServerTlsConfig},
    Request, Response,
};

use crate::consensus_server::ConsensusConfig;
#[allow(unused)]
use crate::{
    consensus_client::ConsensusClient,
    get_addr,
    protos::{
        uplink_deduplication_consensus_server::{
            UplinkDeduplicationConsensus, UplinkDeduplicationConsensusServer,
        },
        ReceptionSetDisseminationRequest, ReceptionSetDisseminationResponse,
        UplinkReceivedDisseminationRequest, UplinkReceivedDisseminationResponse,
    },
    ConsensusCerts, ConsensusError, ConsensusMessage, ConsensusRound, ConsensusState,
};

type ConsensusResult<T> = Result<Response<T>, tonic::Status>;
type Rounds = Arc<RwLock<HashMap<String, Mutex<ConsensusRound>>>>;

//#[derive(Clone, Serialize, Deserialize, Debug)]
//pub struct ConsensusConfig {
//    pub addr: SocketAddr,
//    pub certs: ConsensusCerts,
//}

pub struct MaliciousConsensusServer {
    id: String,
    rounds: Rounds,
    certs: ConsensusCerts,
}

impl MaliciousConsensusServer {
    fn new(id: String, receiver: Receiver<ConsensusMessage>, certs: ConsensusCerts) -> Self {
        let rounds = Arc::new(RwLock::new(HashMap::new()));
        let cloned_rounds = rounds.clone();
        tokio::spawn(MaliciousConsensusServer::consensus_receiver_broadcaster_routine(
            cloned_rounds,
            receiver,
            id.clone(),
            certs.clone(),
        ));
        MaliciousConsensusServer { id, rounds, certs }
    }

    pub fn run_instance(
        id: String,
        config: ConsensusConfig,
    ) -> Result<Sender<ConsensusMessage>, ConsensusError> {
        let server_cert = std::fs::read_to_string(&config.certs.cert_path)?;
        let server_key = std::fs::read_to_string(&config.certs.key_path)?;
        let server_ca = Certificate::from_pem(std::fs::read_to_string(&config.certs.ca_cert_path)?);
        let server_identity = Identity::from_pem(server_cert, server_key);
        let server_tls = ServerTlsConfig::new()
            .identity(server_identity)
            .client_ca_root(server_ca);

        let (sender, receiver) = tokio::sync::mpsc::channel::<ConsensusMessage>(10000);

        tokio::spawn(
            tonic::transport::Server::builder()
                .tls_config(server_tls)
                .map_err(|_| ConsensusError::InvalidTlsConfig)?
                .timeout(Duration::from_millis(300))
                .add_service(UplinkDeduplicationConsensusServer::new(
                    MaliciousConsensusServer::new(id, receiver, config.certs),
                ))
                .serve(config.addr),
        );

        Ok(sender)
    }

    async fn consensus_receiver_broadcaster_routine(
        rounds: Rounds,
        mut receiver: Receiver<ConsensusMessage>,
        id: String,
        certs: ConsensusCerts,
    ) {
        while let Some(msg) = receiver.recv().await {
            let mut nc_set = HashMap::new();
            nc_set.insert(id.clone(), 1);
            let dev_addr: String = msg.dev_addr.clone();

            rounds.write().await.insert(
                dev_addr,
                Mutex::new(ConsensusRound {
                    started_at: Instant::now(),
                    status: ConsensusState::ReceivingDisseminations,
                    nc_list: msg.nc_list.clone(),
                    nc_set,
                    packet: msg.packet.clone(),
                    received_sets: Vec::from([id.clone()]),
                    sender: msg.response,
                }),
            );

            //tokio::time::sleep(Duration::from_millis(100)).await;
            let list = msg.nc_list.clone();
            for nc in list.iter() {
                if nc != &id {
                    let mut client = ConsensusClient::new(format!("https://{}", get_addr(nc)), &certs).await.unwrap();
                    //let mut client = ConsensusClient::new(format!("https://{}:5050", nc), &certs).await.unwrap();
                    match client.broadcast_reception(&msg.dev_addr, &msg.packet, msg.rssi).await {
                        Ok(UplinkReceivedDisseminationResponse { answer }) => {
                            if let Some(answer) = answer {
                                if let Err(_e) = MaliciousConsensusServer::broadcast_reception_handler(&id, nc, &rounds, answer, &certs,).await {
                                    //eprintln!("[{id}]: {e:?}");
                                }
                            }
                        }
                        Err(_e) => {}
                    }
                }
            }
        }
    }

    pub async fn is_round_full(rounds: &Rounds, dev_addr: &str) -> bool {
        if let Some(round) = rounds.read().await.get(dev_addr) {
            let round = round.lock().await;
            round.nc_set.len() == round.nc_list.len()
        } else {
            false
        }
    }

    //pub async fn add_nc_to_set(rounds: &Rounds, dev_addr: &str, nc_id: &str, rssi: i32) -> bool {
    pub async fn add_nc_to_set(rounds: &Rounds, dev_addr: &str, nc_id: &str) -> bool {
        if let Some(round) = rounds.read().await.get(dev_addr) {
            let mut round = round.lock().await;
            let nc_id = nc_id.to_string();
            if !round.nc_list.contains(&nc_id) {
                false
            } else {
                round.nc_set.insert(nc_id.clone(), 1);
                //println!("[{id}]: Adding {nc_id} to the set, new set: {:?}", round.nc_set);
                true
            }
        } else {
            false
        }
    }

    pub async fn select_winner_by_rssi(rounds: &Rounds, pkt_id: &str) -> Option<String> {
        if let Some(round) = rounds.read().await.get(pkt_id) {
            let round = round.lock().await;
            let mut best_rssi = 0;
            let mut best_nc = None;
            for (nc, rssi) in &round.nc_set {
                if *rssi > best_rssi {
                    best_rssi = *rssi;
                    best_nc = Some(nc.clone());
                }
            }
            best_nc
        } else {
            None
        }
    }

    pub async fn is_winner(id: &str, round: &ConsensusRound) -> bool {
        let (_, mic) = round.packet.split_at(round.packet.len() - 4);
        let n = u32::from_le_bytes(mic.try_into().expect("It is always 4 bytes long"));
        let threshold = (round.nc_list.len() as f32) * 0.66;
        println!("{threshold}");

        let mut valid_list: Vec<&String> = round.nc_set.iter().filter(|(_,v)| {
            (**v as f32) > threshold
        }).map(|(k,_)| {k}).collect();
        valid_list.sort();
        let list_len = valid_list.len();
        let name = valid_list[n as usize % list_len];
        //TODO double check

        //let name = round.nc_list[n as usize % round.nc_list.len()].clone();
        println!("Original set: {:?}", round.nc_set);
        println!("Checking winner: {name} - {id}\nList: {:?} - index {}", valid_list, n as usize % list_len);
        name == id
    }

    pub async fn end_round(rounds: &Rounds, dev_addr: &str) -> Option<ConsensusRound> {
        rounds.write().await.remove(dev_addr).map(|round| round.into_inner())
    }

    pub async fn check_round( rounds: &Rounds, r: &UplinkReceivedDisseminationRequest, src: &str, ) -> Result<(), ConsensusError> {
        if let Some(round) = rounds.read().await.get(&r.dev_addr) {
            let round = round.lock().await;
            if round.nc_set.contains_key(src) {
                return Err(ConsensusError::NCAlreadyInSet);
            }
            if round.status != ConsensusState::ReceivingDisseminations {
                return Err(ConsensusError::WrongState);
            }
            if r.hash.len() != 32 {
                return Err(ConsensusError::InvalidHashLength);
            }
            let mut hasher = Sha256::new();
            hasher.update(round.packet.as_slice());
            hasher.update(&r.rssi.to_le_bytes());
            let digest = hasher.finish();
            //println!("Round: {round:?}, request: {r:?}, Checking hash: {:?} == {:?} = {}", digest.as_slice(), r.hash, digest.as_slice() == r.hash);
            if digest.as_slice() == r.hash {
                Ok(())
            } else {
                Err(ConsensusError::InvalidHash)
            }
        } else {
            Err(ConsensusError::NoRound)
        }
    }

    pub fn extract_cn_from_certificate(
        certificates: Option<std::sync::Arc<Vec<CertificateDer<'_>>>>,
    ) -> Option<String> {
        if let Some(certs) = certificates {
            if certs.is_empty() {
                None
            } else {
                let cert = certs .iter().last().expect("Cert should always be present as it is SSL/TLS enabled");
                let pem = X509::from_der(cert).expect("Cert should always be present as it is SSL/TLS enabled");
                pem.subject_name().entries().find(|name| name.object().nid() == Nid::COMMONNAME).map(|name| String::from_utf8_lossy(name.data().as_slice()).into_owned())
            }
        } else {
            None
        }
    }

    pub fn create_dissemination_request(
        dev_addr: &str,
        packet: &[u8],
        rssi: i32,
    ) -> UplinkReceivedDisseminationRequest {
        let mut hasher = Sha256::new();
        hasher.update(packet);
        hasher.update(&rssi.to_le_bytes());
        let hash = hasher.finish().to_vec();

        UplinkReceivedDisseminationRequest {
            dev_addr: dev_addr.to_string(),
            hash,
            rssi,
        }
    }

    pub async fn check_dissemination_set_request(
        name: String,
        r: &ReceptionSetDisseminationRequest,
        round: &mut ConsensusRound,
    ) -> Result<bool, ConsensusError> {
        if round.status != ConsensusState::ReceivingSets {
            return Err(ConsensusError::WrongState);
            //return Err(tonic::Status::aborted("Not in the right state"))
        } else if !round.nc_list.contains(&name) {
            return Err(ConsensusError::NotPartOfRound);
            //return Err(tonic::Status::unauthenticated("Not part of the consensus round"))
        }
        if round.received_sets.contains(&name) {
            return Err(ConsensusError::NCAlreadyInSet);
            //return Err(tonic::Status::aborted("Already in the set"))
        }

        //println!("[{id}]: Checking set request from {name} -- {:?} == {:?} = {}", r.set, round.nc_set, r.set == round.nc_set);

        for nc in r.set.keys() {
            if !round.nc_list.contains(nc)  {
                return Err(ConsensusError::NotPartOfRound);
            }
            if let Some(v) = round.nc_set.get_mut(nc) {
                *v += 1;
            }
        }

        round.received_sets.push(name);
        
        if round.received_sets.len() == round.nc_list.len() {
            round.status = ConsensusState::END;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn broadcast_reception_handler(
        id: &str,
        src: &str,
        rounds: &Rounds,
        r: UplinkReceivedDisseminationRequest,
        certs: &ConsensusCerts,
    ) -> Result<Response<UplinkReceivedDisseminationResponse>, tonic::Status> {
        //println!("{} received a dissemination message from {}", id, src);

        match MaliciousConsensusServer::check_round(rounds, &r, src).await {
            Err(e) => {
                if e == ConsensusError::NCAlreadyInSet {
                    Ok(Response::new(UplinkReceivedDisseminationResponse {
                        answer: None,
                    }))
                } else {
                    Err(tonic::Status::aborted(format!("{:?}", e)))
                }
            }
            Ok(_) => {
                let added = MaliciousConsensusServer::add_nc_to_set(rounds, &r.dev_addr, src).await;
                if !added {
                    return Err(tonic::Status::unauthenticated(
                        "Not part of the consensus round",
                    ));
                }

                let dissemination_request = {
                    let rounds = rounds.read().await;
                    let round = rounds
                        .get(&r.dev_addr)
                        .ok_or(tonic::Status::aborted("No round found"))?;
                    let inner_round = round.lock().await;
                    let packet = &inner_round.packet;
                    MaliciousConsensusServer::create_dissemination_request(
                        &r.dev_addr,
                        packet,
                        *inner_round
                            .nc_set
                            .get(id)
                            .expect("NC id should always be in the set"),
                    )
                };

                if MaliciousConsensusServer::is_round_full(rounds, &r.dev_addr).await {
                    let (nc_list, nc_set) = {
                        let rounds = rounds.read().await;
                        let round = rounds.get(&r.dev_addr).ok_or(tonic::Status::aborted("No round found"))?;
                        let mut round = round.lock().await;

                        round.status = ConsensusState::ReceivingSets;
                        
                        //let mut set = round.nc_set.clone();
                        //let keys = set.keys().cloned().collect::<Vec<String>>();
                        //for k in keys {
                        //    if k != id {
                        //        set.remove(&k);
                        //    }
                        //}
                        let set = HashMap::from([(src.to_string(), 1)]);
                        (round.nc_list.clone(), set)
                    };

                    let rounds = rounds.clone();
                    let id_cloned = id.to_string();
                    let certs_cloned = certs.clone();
                    tokio::spawn(async move {
                        for nc in nc_list {
                            if nc != id_cloned {
                                //let certs = ConsensusCerts {
                                //    cert_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{}/tls/server.crt", id_cloned),
                                //    key_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{}/tls/server.key", id_cloned),
                                //    ca_cert_path: format!("/home/rastafan/Documenti/Dottorato/code/DeLoRaN/lorawan-blockchain/pure_network/crypto-config/peerOrganizations/org1.dlwan.phd/peers/{}/tls/ca.crt", id_cloned),
                                //};

                                let mut client = ConsensusClient::new(format!("https://{}", get_addr(&nc)), &certs_cloned).await.unwrap();
                                //let mut client = ConsensusClient::new( format!("https://{}:5050", nc), &certs_cloned, ).await.unwrap();
                                match client .broadcast_nc_set(r.dev_addr.clone(), nc_set.clone()).await {
                                    Err(_e) => {} //eprintln!("[{id_cloned}]: {:?}", e),
                                    Ok(r) => {
                                        if let Some(answer) = r.answer {
                                            if let Err(_e) = MaliciousConsensusServer::broadcast_nc_set_handler( &id_cloned, nc, &rounds, answer, ) .await {
                                                //eprintln!("[{id_cloned}]: {:?}", e)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
                Ok(Response::new(UplinkReceivedDisseminationResponse {
                    answer: Some(dissemination_request),
                }))
            }
        }
    }

    pub async fn broadcast_nc_set_handler(
        id: &str,
        src: String,
        rounds: &Rounds,
        r: ReceptionSetDisseminationRequest,
    ) -> ConsensusResult<ReceptionSetDisseminationResponse> {
        //println!("{} received a set message from {}", id, src);

        let result = {
            let rounds = rounds.read().await;
            let mut round = rounds .get(&r.dev_addr).ok_or(tonic::Status::aborted("No round found"))?.lock().await;
            //let nc_set = round.nc_set.clone();
            let nc_set = HashMap::from([(src.clone(), 1)]);


            match MaliciousConsensusServer::check_dissemination_set_request(src, &r, &mut round).await {
                Ok(ended) => {
                    let mut winner = false;
                    if ended {
                        winner = MaliciousConsensusServer::is_winner(id, &round).await;
                    }
                    Ok((nc_set, ended, winner))
                }
                Err(e) => Err(e),
            }
        };

        match result {
            Ok((nc_set, ended, winner)) => {
                if ended {
                    if let Some(round) = MaliciousConsensusServer::end_round(rounds, &r.dev_addr).await {
                        if let Err(e) = round.sender.send(winner) {
                            println!("Unable to send consensus answer, it was {e}")
                        };
                    }
                }
                Ok(Response::new(ReceptionSetDisseminationResponse {
                    answer: Some(ReceptionSetDisseminationRequest {
                        dev_addr: r.dev_addr,
                        set: nc_set,
                    }),
                }))
            }
            Err(e) => {
                if e == ConsensusError::NCAlreadyInSet {
                    Ok(Response::new(ReceptionSetDisseminationResponse {
                        answer: None,
                    }))
                } else {
                    Err(tonic::Status::aborted(format!("{:?}", e)))
                }
            }
        }
    }
}

#[tonic::async_trait]
impl UplinkDeduplicationConsensus for MaliciousConsensusServer {
    async fn broadcast_reception(&self,request: Request<UplinkReceivedDisseminationRequest>,) -> ConsensusResult<UplinkReceivedDisseminationResponse> {
        let name = MaliciousConsensusServer::extract_cn_from_certificate(request.peer_certs());
        if let Some(name) = name {
            //println!("{} received a broadcast reception from {}", self.id, name);
            let rounds = &self.rounds;
            MaliciousConsensusServer::broadcast_reception_handler( &self.id, &name, rounds, request.into_inner(), &self.certs, ).await
        } else {
            Err(tonic::Status::unauthenticated("No certificate provided"))
        }
    }

    async fn broadcast_nc_set(&self,request: Request<ReceptionSetDisseminationRequest>,) -> ConsensusResult<ReceptionSetDisseminationResponse> {
        let name = MaliciousConsensusServer::extract_cn_from_certificate(request.peer_certs());
        if let Some(name) = name {
            //println!("{} received a broadcast nc_set from {}", self.id, name);
            let r = request.into_inner();
            MaliciousConsensusServer::broadcast_nc_set_handler(&self.id, name, &self.rounds, r).await
        } else {
            Err(tonic::Status::unauthenticated("No certificate provided"))
        }
    }
}
