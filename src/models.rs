// use libp2p::swarm::behaviour::NetworkBehaviour;
use libp2p::{mdns, swarm::{NetworkBehaviour, behaviour}, floodsub::Floodsub};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

pub type Recipes = Vec<Recipe>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    id: usize,
    name: String,
    ingredients: String,
    Instructions: String,
    public: bool,
}


pub enum EventType {
    Input(String),
    Response(ListResponse),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ListMode {
    ALL,
    ONE(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    mode: ListMode,
    data: Recipes,
    receiver: String,
}


#[derive(NetworkBehaviour)]
pub struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: mdns::tokio::Behaviour,
    // #[behaviour(ignore)]
    // resp_sender: mpsc::UnboundedSender<ListResponse>,
}


