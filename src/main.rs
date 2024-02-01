use libp2p::core::transport::upgrade::{self, Multiplex};
use libp2p::identity::Keypair;
use log::info;
use once_cell::sync::Lazy;

use libp2p::{identity, PeerId, Transport};
use libp2p::floodsub::Topic;
use libp2p::tcp::tokio::{Tcp, TcpStream, self as libp2p_tokio};
use libp2p::yamux;

mod models;
use models as Mod;
use libp2p::noise::Config as NoiseConfig;
// use tokio::sync::mpsc;

const STORAGE_FILE_PATH: &str = "./recipes.json";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from_public_key(&KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

#[tokio::main]
async fn main() {

    pretty_env_logger::init();
    info!("Peer Id: {}", PEER_ID.clone());
    
    
}
