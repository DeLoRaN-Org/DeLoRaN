#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::{exec_bridge::{BlockchainExeClient, BlockchainExeConfig}, BlockchainClient};

    fn get_epoch_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }
    
    #[tokio::test]
    async fn create_convergence_flag() {
        let bc_config = BlockchainExeConfig {
            orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
            channel_name: "lorawan".to_string(),
            chaincode_name: "lorawan".to_string(),
            orderer_ca_file_path: None,
        };

        let client = *BlockchainExeClient::from_config(&bc_config).await.unwrap();
        
        let before = get_epoch_ms();
        client.create_flag().await.expect("Error creating flag");
        let after = get_epoch_ms();

        let creation_flag_time = after - before;
        println!("Start creating flag for convergence experiment: {}", before);
        println!("Stop experiment:                                {}", after);
        println!("Creation flag time:                             {:?}", creation_flag_time);
        let _end: Option<u32> = None;
    }
    
    #[tokio::test]
    async fn clear_convergence_test() {
        let bc_config = BlockchainExeConfig {
            orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
            channel_name: "lorawan".to_string(),
            chaincode_name: "lorawan".to_string(),
            orderer_ca_file_path: None,
        };

        let client = *BlockchainExeClient::from_config(&bc_config).await.unwrap();
        
        let before = get_epoch_ms();
        client.clear_flag().await.expect("Error clearing flag");
        let after = get_epoch_ms();

        let creation_flag_time = after - before;
        println!("Clearing flag time: {:?}", creation_flag_time);
        let _end: Option<u32> = None;
    }
    
    
    #[tokio::test]
    async fn read_convergence_flag() {
        let bc_config = BlockchainExeConfig {
            orderer_addr: "orderer1.orderers.dlwan.phd:6050".to_string(),
            channel_name: "lorawan".to_string(),
            chaincode_name: "lorawan".to_string(),
            orderer_ca_file_path: None,
        };

        let client = *BlockchainExeClient::from_config(&bc_config).await.unwrap();
        
        let before = get_epoch_ms();

        let mut flag = String::from("notaflag");
        let mut i = 0;
        while flag == "notaflag" {
            println!("Reading flag for convergence experiment: {}", i);
            i += 1;
            flag = client.get_flag().await.expect("Error reading flag");
        }
        let after = get_epoch_ms();

        let read_flag_time = after - before;
        
        println!("Start reading flag for convergence experiment: {}", before);
        println!("Stop experiment:                               {}", after);
        println!("Obtainer flag {flag}, read flag time:          {:?}", read_flag_time);

        let _end: Option<u32> = None;
    }
}