use application_server::application_server::{ApplicationServer, ApplicationServerConfig};
//use application_server::application_server::ApplicationServer;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
///The Network Controller implementation for DistributedLoRaWAN
struct Args {
    /// Path of the configuration JSON file.
    #[clap(short, long, value_parser)]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let _args = Args::parse();
    
    static APPLICATION_SERVER_CONFIG: ApplicationServerConfig = ApplicationServerConfig {
        tcp_receive_port: 1680,
    };

    tokio::spawn(async move {
        let application_server = ApplicationServer::init(&APPLICATION_SERVER_CONFIG).await;
        application_server.routine().await.unwrap();
    }).await.unwrap();

}
