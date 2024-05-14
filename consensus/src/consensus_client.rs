use std::collections::HashMap;

use tonic::{transport::{Certificate, Channel, Identity}, Request};

use crate::{consensus_server::ConsensusServer, protos::{uplink_deduplication_consensus_client::UplinkDeduplicationConsensusClient, ReceptionSetDisseminationRequest, ReceptionSetDisseminationResponse, UplinkReceivedDisseminationResponse}, ConsensusCerts, ConsensusError};


pub struct ConsensusClient {
    consensus_client: UplinkDeduplicationConsensusClient<Channel>,
}

impl ConsensusClient {
    pub async fn new(addr: String, certs: ConsensusCerts) -> Result<Self, ConsensusError> {
        let client_cert = std::fs::read_to_string(certs.cert_path)?;
        let client_key = std::fs::read_to_string(certs.key_path)?;
        let certificate = Certificate::from_pem(std::fs::read_to_string(certs.ca_cert_path)?);
        
        let client_identity = Identity::from_pem(client_cert, client_key);
        let tls_config = tonic::transport::ClientTlsConfig::new().ca_certificate(certificate).identity(client_identity);
        let channel = Channel::from_shared(addr).map_err(|_| ConsensusError::InvalidUri)?.tls_config(tls_config).map_err(|_| ConsensusError::InvalidTlsConfig)?.connect().await?;

        Ok(ConsensusClient {
            consensus_client: UplinkDeduplicationConsensusClient::new(channel),
        })
    }

    pub async fn broadcast_reception(&mut self, dev_addr: &str, packet: &[u8], rssi: i32) -> Result<UplinkReceivedDisseminationResponse, tonic::Status> {
        let request = ConsensusServer::create_dissemination_request(dev_addr, packet, rssi);
        let response = self.consensus_client.broadcast_reception(Request::new(request)).await?;
        Ok(response.into_inner())
    }

    pub async fn broadcast_nc_set(&mut self, dev_addr: String, set: HashMap<String, i32>) -> Result<ReceptionSetDisseminationResponse, tonic::Status> {
        let request = ReceptionSetDisseminationRequest {
            dev_addr,
            set,
        };
        let response = self.consensus_client.broadcast_nc_set(Request::new(request)).await?;
        Ok(response.into_inner())
    }
}
