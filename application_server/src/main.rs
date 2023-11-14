use application_server::application_server::ApplicationServer;
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
    
    //tokio::spawn(async move {
    //let application_server = ApplicationServer::init().await;
    //application_server.routine().await.unwrap();
    //}).await.unwrap();

}
